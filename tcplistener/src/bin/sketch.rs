// chrono = "0.4.31"

use std::{error::Error, net::UdpSocket, ops::Range};

use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};

const NTP_PACKET_SIZE: usize = 48;
const TX_SECONDS: Range<usize> = 40..44;

fn main() -> Result<(), Box<dyn Error>> {
    // NB server seems to not always respond so we may want to retry with a timeout
    // ok so some generic server
    let ntp_server = "pool.ntp.org:123";

    // `0.0.0.0` binds to a network interface that can interact with the internet
    // we can't use `127.0.0.1` here because that's a loopback that has not access to the internet
    // `:0` means let the OS assign us a random free port
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    dbg!(&sock);
    let mut request = [0u8; NTP_PACKET_SIZE];
    // this magic number means
    // - use NTPv3
    // - we are a client
    // ok...
    request[0] = 0x1b;

    // it sometimes does not sends all the bytes!!
    let written = sock.send_to(&request, ntp_server)?;
    // why this assert
    assert_eq!(NTP_PACKET_SIZE, written);

    // reuse buffer
    // why that? such a small buffer of 48 bytes
    let mut response = request;
    let (read, peer) = sock.recv_from(&mut response)?;
    assert_eq!(NTP_PACKET_SIZE, read);

    // take note of the IP address
    // ok [src/bin/sketch.rs:38] peer = 192.121.108.100:123
    // no DNS resolution, that IP will change
    // NTP server can go down and the add can be updated on the mcu
    dbg!(peer);

    // how does this fills? ah ok this is why we have range 40..44
    // the packet is 48 bytes
    // 1 bytes == header with protocol
    // then bytes 40..44 --> represents the ntp time in seconds
    // # of seconds that have elapsed from NTP epoch = 3_906_531_996 etc, like the stuff from 1970's
    let transmit_seconds = u32::from_be_bytes(response[TX_SECONDS].try_into().unwrap());
    dbg!(transmit_seconds);
    let now = Local::now();
    let offset = now.offset();

    // 1900-01-01T00:00:00+00:00
    // why do we declare this epoch from back in the days...
    // because it's NTP epoch
    let ntp_epoch = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(1900, 1, 1).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    );
    let ntp_dt = DateTime::<Local>::from_naive_utc_and_offset(ntp_epoch, *offset)
        + Duration::seconds(transmit_seconds.into());
    // this has nothing to do with 1900
    println!("sys time: {now}");
    println!("NTP time: {ntp_dt}");

    Ok(())
}
