    use super::*;
    #[std::prelude::v1::test]
    fn pipe_send_and_shutdown_do_not_wait_for_a_reader() {
        let dir = std::env::temp_dir().join("inkling-agent").join(format!("audit-pipe-{}", crate::store::new_id()));
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "exec sleep 10"]);
        let (mut proc, _stdout) = Proc::spawn(cmd, &dir.join("brain.log")).unwrap();
        let value = serde_json::json!({"payload": "x".repeat(1_000_000)});
        let start = Instant::now();
        proc.send(&value).unwrap();
        let elapsed = start.elapsed();
        eprintln!("AUDIT pipe: send_ms={:.1}", elapsed.as_secs_f64() * 1000.);
        let stopping = Instant::now();
        proc.stop();
        assert!(elapsed < Duration::from_millis(750), "pipe blocked caller for {elapsed:?}");
        assert!(stopping.elapsed() < Duration::from_millis(750), "shutdown missed its deadline");
    }

    #[std::prelude::v1::test]
    fn stalled_writer_has_bounded_backpressure() {
        let dir = std::env::temp_dir().join("inkling-agent").join(format!("pipe-capacity-{}", crate::store::new_id()));
        let mut cmd = Command::new("/bin/sh"); cmd.args(["-c", "exec sleep 10"]);
        let (mut proc, _stdout) = Proc::spawn(cmd, &dir.join("brain.log")).unwrap();
        let value = json!({"payload": "x".repeat(1_000_000)});
        let mut accepted = 0;
        while accepted < 40 && proc.send(&value).is_ok() { accepted += 1; }
        proc.stop();
        assert!(accepted <= 33, "32 queued plus at most one being written, got {accepted}");
        assert!(accepted > 0);
    }

    #[std::prelude::v1::test]
    fn a_broken_input_pipe_reports_failure_even_when_stdout_stays_open() {
        let dir = std::env::temp_dir().join("inkling-agent").join(format!("pipe-failure-{}", crate::store::new_id()));
        let mut cmd = Command::new("/bin/sh"); cmd.args(["-c", "exec 0<&-; exec sleep 10"]);
        let (mut proc, _stdout) = Proc::spawn(cmd, &dir.join("brain.log")).unwrap();
        let errors = proc.failed.take().unwrap();
        proc.send(&json!({"payload": "x".repeat(1_000_000)})).unwrap();
        let failure = errors.recv_timeout(Duration::from_secs(2));
        proc.stop();
        assert!(failure.is_ok(), "the reader's EOF must not be needed to discover a broken writer");
    }
