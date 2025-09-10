use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use std::process::Command;

pub struct ConnectivityTest;

impl ConnectivityTest {
    pub fn test_tcp_connection(hostname: &str, port: u16) -> bool {
        let address = format!("{}:{}", hostname, port);
        
        match address.to_socket_addrs() {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    TcpStream::connect_timeout(&addr, Duration::from_secs(5)).is_ok()
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }
    
    pub fn connect_ssh(host_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Stdio;
        
        let mut cmd = Command::new("ssh");
        cmd.arg(host_name)
           .stdin(Stdio::inherit())
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit());
        
        let status = cmd.status()?;
        
        if !status.success() {
            return Err(format!("SSH connection failed with exit code: {:?}", status.code()).into());
        }
        
        Ok(())
    }
}