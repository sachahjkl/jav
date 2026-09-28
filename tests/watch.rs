#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const WRAPPER: &str = r#"#!/bin/sh
if [ -f fail ]; then
    echo failed >> events
    exit 7
fi
echo "start $$" >> events
sh -c '
    trap '\''echo "child-stop $$" >> events; exit 0'\'' TERM INT
    echo "child-start $$" >> events
    while :; do sleep 0.1; done
' &
child=$!
trap 'wait "$child"; echo "stop $$" >> events; exit 0' TERM INT
wait "$child"
"#;

struct Watch {
    child: Child,
    root: tempfile::TempDir,
}

impl Watch {
    fn start(fail: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "pom.xml", "<project/>");
        write(
            root.path(),
            "module/src/main/java/Main.java",
            "class Main {}",
        );
        write(
            root.path(),
            "module/src/main/resources/application.properties",
            "value=1",
        );
        write(root.path(), "mvnw", WRAPPER);
        // A non-executable wrapper must run through sh.
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.path().join("mvnw"), fs::Permissions::from_mode(0o644)).unwrap();
        if fail {
            write(root.path(), "fail", "");
        }
        let log = fs::File::create(root.path().join("watch.log")).unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_jav"))
            .current_dir(root.path())
            .args(["run", "--watch", "--no-build"])
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        Self { child, root }
    }

    fn events(&self) -> String {
        fs::read_to_string(self.root.path().join("events")).unwrap_or_default()
    }

    fn wait_events(&mut self, event: &str, count: usize) {
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            if self
                .events()
                .lines()
                .filter(|line| line.starts_with(event))
                .count()
                >= count
            {
                return;
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "watch exited: {}",
                self.log()
            );
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {count} {event} events: {}\n{}",
                self.events(),
                self.log()
            );
            thread::sleep(Duration::from_millis(25));
        }
    }

    fn log(&self) -> String {
        fs::read_to_string(self.root.path().join("watch.log")).unwrap_or_default()
    }

    fn stop(&mut self, signal: i32) {
        // The signal targets only this test's watcher.
        assert_eq!(unsafe { libc::kill(self.child.id() as i32, signal) }, 0);
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "watch failed: {}", self.log());
                return;
            }
            assert!(
                Instant::now() < deadline,
                "watch did not stop: {}",
                self.log()
            );
            thread::sleep(Duration::from_millis(25));
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            unsafe {
                libc::kill(self.child.id() as i32, libc::SIGINT);
            }
            let deadline = Instant::now() + Duration::from_secs(3);
            while self.child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(25));
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        // Clean up recorded fixture processes if a failed assertion interrupted shutdown.
        let events = self.events();
        for line in events.lines() {
            if line.starts_with("start ") || line.starts_with("child-start ") {
                if let Some(pid) = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|pid| pid.parse::<i32>().ok())
                {
                    let stopped = if line.starts_with("child-start ") {
                        format!("child-stop {pid}")
                    } else {
                        format!("stop {pid}")
                    };
                    if !events.lines().any(|event| event == stopped) {
                        unsafe {
                            libc::kill(pid, libc::SIGKILL);
                        }
                    }
                }
            }
        }
    }
}

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn restarts_for_module_inputs_and_stops_the_process_tree() {
    let mut watch = Watch::start(false);
    watch.wait_events("child-start ", 1);
    for path in [
        "README.md",
        "module/build/generated.java",
        "module/target/Main.java",
        "module/out/Main.java",
        ".gradle/test.gradle",
        ".git/test.gradle",
        ".jav-build-test/Main.java",
    ] {
        write(watch.root.path(), path, "ignored");
    }
    thread::sleep(Duration::from_millis(1200));
    assert_eq!(
        watch
            .events()
            .lines()
            .filter(|line| line.starts_with("start "))
            .count(),
        1
    );

    write(
        watch.root.path(),
        "module/src/main/java/Main.java",
        "class Main { int changed; }",
    );
    watch.wait_events("child-start ", 2);
    watch.wait_events("child-stop ", 1);
    watch.wait_events("stop ", 1);
    fs::remove_file(
        watch
            .root
            .path()
            .join("module/src/main/resources/application.properties"),
    )
    .unwrap();
    watch.wait_events("child-start ", 3);
    write(
        watch.root.path(),
        "scripts/conventions.gradle.kts",
        "// changed build script",
    );
    watch.wait_events("child-start ", 4);
    watch.stop(libc::SIGINT);
    assert_eq!(
        watch
            .events()
            .lines()
            .filter(|line| line.starts_with("child-stop "))
            .count(),
        4,
        "{}",
        watch.events()
    );
    assert_eq!(
        watch
            .events()
            .lines()
            .filter(|line| line.starts_with("stop "))
            .count(),
        4,
        "{}",
        watch.events()
    );
    for line in watch
        .events()
        .lines()
        .filter(|line| line.starts_with("child-start "))
    {
        let pid: i32 = line.split_whitespace().nth(1).unwrap().parse().unwrap();
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            -1,
            "child {pid} remains alive"
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
}

#[test]
fn retries_after_a_failed_run_when_inputs_change() {
    let mut watch = Watch::start(true);
    watch.wait_events("failed", 1);
    fs::remove_file(watch.root.path().join("fail")).unwrap();
    write(
        watch.root.path(),
        "module/src/main/java/Main.java",
        "class Main { int fixed; }",
    );
    watch.wait_events("child-start ", 1);
    watch.stop(libc::SIGTERM);
    assert!(watch.events().contains("child-stop "));
    assert!(watch.events().contains("stop "));
}
