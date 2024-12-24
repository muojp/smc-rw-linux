#[cfg(test)]
mod tests {
    use super::super::*;
    use mockall::mock;
    use std::mem;
    use std::panic;

    #[test]
    fn test1() {
        let mut mock_rw = MockIoPortRw::new();
        mock_rw.expect_delayed_inb()
            .return_const(0x00);
        let smc_rw = DefaultSmcRw::new(mock_rw);
        assert_eq!(-1, smc_rw.wait_read());
    }

    #[test]
    fn test2() {
        let mut mock_rw = MockIoPortRw::new();
        mock_rw.expect_delayed_inb()
            .return_const(0x01);
        let smc_rw = DefaultSmcRw::new(mock_rw);
        assert_eq!(0, smc_rw.wait_read());
    }

    #[test]
    fn test3() {
        let mut mock_rw = MockIoPortRw::new();
        mock_rw.expect_delayed_inb()
            .return_const(0x05);
        let smc_rw = DefaultSmcRw::new(mock_rw);
        assert_eq!(0, smc_rw.wait_read());
    }

    #[test]
    fn test4() {
        let mut mock_rw = MockIoPortRw::new();
        mock_rw.expect_delayed_inb()
            .returning(|us, _| if us == 0x40 { 0x01 } else { 0x00 });
        let smc_rw = DefaultSmcRw::new(mock_rw);
        assert_eq!(0, smc_rw.wait_read());
    }

    #[test]
    fn test5() {
        let mut mock_rw = MockIoPortRw::new();
        mock_rw.expect_delayed_inb()
            .returning(|us, _| if us == 0x10000 { 0x01 } else { 0x00 });
        let smc_rw = DefaultSmcRw::new(mock_rw);
        assert_eq!(0, smc_rw.wait_read());
    }

    #[test]
    fn test6() {
        let mut mock_rw = MockIoPortRw::new();
        mock_rw.expect_delayed_inb()
            .returning(|us, _| if us == 0x20000 { 0x01 } else { 0x00 });
        let smc_rw = DefaultSmcRw::new(mock_rw);
        assert_eq!(-1, smc_rw.wait_read());
    }

    #[test]
    fn test7() {
        let mut mock_rw = MockIoPortRw::new();
        mock_rw.expect_delayed_inb().return_const(0x04);
        mock_rw.expect_delayed_outb().return_const(());
        let smc_rw = DefaultSmcRw::new(mock_rw);
        assert!(smc_rw.send_byte(0, 1).is_ok());
    }

    #[test]
    fn test8() {
        let mut mock_rw = MockIoPortRw::new();
        mock_rw.expect_delayed_inb().return_const(0x04);
        mock_rw.expect_delayed_outb().return_const(());
        let smc_rw = DefaultSmcRw::new(mock_rw);
        assert!(smc_rw.send_argument([0, 1, 2, 3]).is_ok());
    }

    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, Ordering};

    #[test]
    fn test9() {
        let last_cmd = Arc::new(AtomicU8::new(0));
        let last_cmd_for_inb = last_cmd.clone();

        let mut mock_rw = MockIoPortRw::new();
        mock_rw.expect_delayed_inb().returning(move |_, _| {
            let cmd = last_cmd_for_inb.load(Ordering::SeqCst);
            if cmd == 15 { 0x00 } else { 0x04 }
        });
        mock_rw.expect_delayed_outb().returning(move |_, cmd, _| {
            last_cmd.store(cmd, Ordering::SeqCst);
        });
        let smc_rw = DefaultSmcRw::new(mock_rw);
        // send_argument results in an error when internal send_byte fails
        match smc_rw.send_argument([0, 1, 15, 3]) {
            Ok(()) => panic!("Expected an error."),
            Err((2, _)) => (),
            _ => panic!("Expected certain error idx")
        }
    }

    mock! {
        SmcTest {}
        impl SmcPrimitive for SmcTest {
            type IoPort = MockIoPortRw;
            fn new(io_port_rw: MockIoPortRw) -> Self;
            fn wait_read(&self) -> libc::c_int;
            fn send_byte(&self, cmd: u8, port: u16) -> Result<libc::c_int, String>;
            fn send_argument(&self, key: [u8; SMC_KEY_NAME_LEN]) -> Result<(), (usize, String)>;
            fn recv_byte(&self) -> Result<u8, ()>;
        }
    }
    // actual test targets are in default implementation for SmcOperation trait
    impl SmcOperation for MockSmcTest {}

    use std::sync::Mutex;
    use once_cell::sync::Lazy;

    static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn setup_mock_smc() -> MockSmcTest {
        // Setting up mock context requires serialization. Otherwise, results in flaky tests.
        let _guard = TEST_MUTEX.lock().unwrap();
        let mock_io = MockIoPortRw::new();
        let context = MockSmcTest::new_context();
        context.expect().return_once(|_| MockSmcTest::default());
        MockSmcTest::new(mock_io)
    }

    #[test]
    fn test10() {
        let mut smc_rw = setup_mock_smc();
        let mut buf = [0u8; 255];

        smc_rw.expect_send_byte().return_const(Ok(0));
        smc_rw.expect_recv_byte().return_const(Ok(0));
        smc_rw.expect_send_argument().return_const(Ok(()));
        smc_rw.expect_wait_read().return_const(0);
        assert!(smc_rw.read_smc(0x00, [0x01, 0x02, 0x03, 0x04], &mut buf).is_ok());
    }

    #[test]
    fn test11() {
        let mut smc_rw = setup_mock_smc();
        let mut buf = [0u8; 255];

        smc_rw.expect_send_byte().return_const(Err(String::from("err_send_byte")));
        smc_rw.expect_recv_byte().return_const(Ok(0));
        smc_rw.expect_send_argument().return_const(Ok(()));
        smc_rw.expect_wait_read().return_const(0);
        match smc_rw.read_smc(0x00, [0x01, 0x02, 0x03, 0x04], &mut buf) {
            Err(s) => assert_eq!("[1, 2, 3, 4]: read arg failed", s),
            Ok(_) => panic!()
        }
    }

    #[test]
    fn test12() {
        let mut smc_rw = setup_mock_smc();
        let mut buf = [0u8; 255];

        smc_rw.expect_send_byte().return_const(Ok(0));
        smc_rw.expect_recv_byte().return_const(Ok(0));
        smc_rw.expect_send_argument().return_const(Err((0, String::from("err_send_argument"))));
        smc_rw.expect_wait_read().return_const(0);
        match smc_rw.read_smc(0x00, [0x01, 0x02, 0x03, 0x04], &mut buf) {
            Err(s) => assert_eq!("[1, 2, 3, 4]: read arg failed", s),
            Ok(_) => panic!()
        }
    }

    #[test]
    fn test13() {
        let mut smc_rw = setup_mock_smc();
        let mut buf = [0u8; 255];

        smc_rw.expect_send_byte().returning(|_, port| if port == SMC_CMD_PORT { Ok(0) } else { Err(String::from("err"))});
        smc_rw.expect_recv_byte().return_const(Ok(0));
        smc_rw.expect_send_argument().return_const(Ok(()));
        smc_rw.expect_wait_read().return_const(0);
        match smc_rw.read_smc(0x00, [0x01, 0x02, 0x03, 0x04], &mut buf) {
            Err(s) => assert_eq!("[1, 2, 3, 4]: read len failed", s),
            Ok(_) => panic!()
        }
    }

    #[test]
    fn test14() {
        let mut smc_rw = setup_mock_smc();
        let mut long_buf = [0u8; 256];

        smc_rw.expect_send_byte().return_const(Ok(0));
        smc_rw.expect_recv_byte().return_const(Ok(0));
        smc_rw.expect_send_argument().return_const(Ok(()));
        smc_rw.expect_wait_read().return_const(0);
        match smc_rw.read_smc(0x00, [0x01, 0x02, 0x03, 0x04], &mut long_buf) {
            Err(s) => assert_eq!("data len limit exceeded", s),
            Ok(_) => panic!()
        }
    }

    #[test]
    fn test15() {
        let mut smc_rw = setup_mock_smc();
        let mut buf = [0u8; 255];

        smc_rw.expect_send_byte().return_const(Ok(0));
        smc_rw.expect_recv_byte().return_const(Ok(0));
        smc_rw.expect_send_argument().return_const(Ok(()));
        smc_rw.expect_wait_read().return_const(-1);
        match smc_rw.read_smc(0x00, [0x01, 0x02, 0x03, 0x04], &mut buf) {
            Err(s) => assert_eq!("[1, 2, 3, 4]: read data 0 failed", s),
            Ok(_) => panic!()
        }
    }

    #[test]
    fn test16() {
        let mut smc_rw = setup_mock_smc();
        let mut data = SmcData { data_len: 0, data_type: [0u8; 4], flags: 0};
        let mut data_buf = [0u8; 16];

        smc_rw.expect_send_byte().return_const(Ok(0));
        smc_rw.expect_recv_byte().return_const(Ok(7));
        smc_rw.expect_send_argument().return_const(Ok(()));
        smc_rw.expect_wait_read().return_const(0);
        let buf = unsafe { std::slice::from_raw_parts_mut(&mut data as *mut SmcData as *mut u8, mem::size_of::<SmcData>()) };
        match smc_rw.read_smc(SMC_GET_KEY_TYPE_CMD, [0x01, 0x02, 0x03, 0x04], buf) {
            Err(s) => panic!("{}", s),
            Ok(_) => {
                let buf = unsafe { std::slice::from_raw_parts_mut(&mut data as *mut SmcData as *mut u8, mem::size_of::<SmcData>()) };
                assert_eq!("[7, 7, 7, 7, 7, 7]", format!("{:?}", buf));
            }
        }
        data_buf.fill(0x55); // reset buffer content. 0x55 == 85
        data.data_len = 14;
        match smc_rw.read_smc(SMC_GET_KEY_TYPE_CMD, [0x01, 0x02, 0x03, 0x04], &mut data_buf[0..data.data_len as usize]) {
            Err(s) => panic!("{}", s),
            Ok(_) => {
                assert_eq!("[7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 85, 85]", format!("{:?}", data_buf));
            }
        }

        let hook = panic::take_hook();
        panic::set_hook(Box::new(|_| {
            
        }));
        data.data_len = 16;
        let result = std::panic::catch_unwind(|| {
            let mut short_buf = [0u8; 8];
            smc_rw.read_smc(SMC_GET_KEY_TYPE_CMD, [0x01, 0x02, 0x03, 0x04], &mut short_buf[0..data.data_len as usize])
        });
        panic::set_hook(hook);

        assert!(result.is_err());
        match result {
            Err(panic_msg) => {
                if let Some(msg) = panic_msg.downcast_ref::<String>() {
                    assert_eq!("range end index 16 out of range for slice of length 8", msg);
                }
            }
            Ok(_) => panic!("should receive an error")
        }
    }
}
