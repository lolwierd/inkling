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