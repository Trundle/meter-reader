use bluer::{gatt::remote::Characteristic, Adapter, AdapterEvent, Address, Device};
use chrono::{Duration, Local, TimeZone};
use clap::Parser;
use futures::{pin_mut, StreamExt};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use meterreader_models::{MeterSampleValue, MeterSectionInfo, MeterValue};

// 0000fd3d-0000-1000-8000-00805f9b34fb
const ADVERTISEMENT_SERVICE_UUID: uuid::Uuid =
    uuid::Uuid::from_u128(0x0000_fd3d_0000_1000_8000_0080_5f9b_34fb_u128);

// cba20d00-224d-11e6-9fb8-0002a5d5c51b
const SERVICE_UUID: uuid::Uuid =
    uuid::Uuid::from_u128(0xcba2_0d00_224d_11e6_9fb8_0002_a5d5_c51b_u128);

// cba20002-224d-11e6-9fb8-0002a5d5c51b
const WRITE_CHAR_UUID: uuid::Uuid =
    uuid::Uuid::from_u128(0xcba2_0002_224d_11e6_9fb8_0002_a5d5_c51b_u128);

// cba20003-224d-11e6-9fb8-0002a5d5c51b
const READ_CHAR_UUID: uuid::Uuid =
    uuid::Uuid::from_u128(0xcba2_0003_224d_11e6_9fb8_0002_a5d5_c51b_u128);

const RESPONSE_OK: u8 = 1;
const CMD_SET_TIME: u8 = 5;
const CMD_READ_INDEX_INFO: u8 = 59;
const CMD_READ_SAMPLE_INFO: u8 = 60;

const SAMPLE_COUNT: u8 = 6;

struct Meter {
    device: Device,
    read_char: Option<Characteristic>,
    write_char: Option<Characteristic>,
}

impl Meter {
    fn new(adapter: &Adapter, addr: Address) -> bluer::Result<Meter> {
        Ok(Meter {
            device: adapter.device(addr)?,
            read_char: None,
            write_char: None,
        })
    }

    async fn connect(&mut self) -> bluer::Result<()> {
        if self.read_char.is_none() {
            self.device.connect().await?;
            if let Some((read_char, write_char)) = find_characteristics(&self.device).await? {
                self.read_char = Some(read_char);
                self.write_char = Some(write_char);
            }
        }

        Ok(())
    }

    pub async fn read_section_info(&mut self) -> bluer::Result<Option<MeterSectionInfo>> {
        let mut cmd = gen_cmd(CMD_READ_INDEX_INFO, 1);
        cmd[3] = 0;
        let response = self.exec(&cmd).await?;
        Ok(MeterSectionInfo::from_response(&response))
    }

    pub async fn read_samples(
        &mut self,
        section_info: &MeterSectionInfo,
        duration: Option<Duration>,
    ) -> bluer::Result<Vec<MeterSampleValue>> {
        let mut result = Vec::with_capacity(section_info.data_length.into());

        let mut all_iter;
        let mut last_n_iter;
        let samples: &mut dyn Iterator<Item = u16> = {
            all_iter = (0..(section_info.data_length / u16::from(SAMPLE_COUNT))
                * u16::from(SAMPLE_COUNT))
                .step_by(SAMPLE_COUNT.into());
            if let Some(duration) = duration {
                let samples_wanted: usize =
                    <i64 as TryInto<usize>>::try_into(duration.num_seconds()).unwrap()
                        / usize::from(section_info.interval);
                let sample_count = usize::from(SAMPLE_COUNT);
                last_n_iter = all_iter
                    .rev()
                    .take((samples_wanted + sample_count - 1) / sample_count)
                    .rev();
                &mut last_n_iter
            } else {
                &mut all_iter
            }
        };
        for i in samples {
            let mut cmd = gen_cmd(CMD_READ_SAMPLE_INFO, 4);
            cmd[3] = 0;
            cmd[4] = (i >> 8) as u8;
            cmd[5] = (i & 0xff) as u8;
            cmd[6] = SAMPLE_COUNT;
            let response = self.exec(&cmd).await?;
            if let Some(mut samples) = MeterSampleValue::from_response(&response) {
                result.append(&mut samples);
            }
        }

        Ok(result)
    }

    pub async fn set_time(&mut self) -> bluer::Result<()> {
        let mut cmd = gen_cmd(CMD_SET_TIME, 10);
        let i = cmd.len() - 10;
        cmd[i] = 3;
        cmd[i + 1] = 0;
        for (j, byte) in Local::now().timestamp().to_be_bytes().iter().enumerate() {
            cmd[i + 2 + j] = *byte;
        }
        let response = self.exec(&cmd).await?;
        if response.is_empty() || response[0] != RESPONSE_OK {
            println!("[WARNING] Got non-okay response when setting time");
        }
        Ok(())
    }

    async fn exec(&mut self, cmd: &[u8]) -> bluer::Result<Vec<u8>> {
        self.connect().await?;
        if let Some(read_char) = &self.read_char {
            let mut notify_io = read_char.notify_io().await?;
            let mut buf = vec![0; notify_io.mtu()];
            let read_future = notify_io.read(&mut buf);

            let mut write_io = self.write_char.as_ref().unwrap().write_io().await?;
            let _ = write_io.write(cmd).await?;
            drop(write_io);

            let read = read_future.await?;
            drop(notify_io);
            buf.truncate(read);
            Ok(buf)
        } else {
            Ok(vec![])
        }
    }

    pub async fn disconnect(&mut self) -> bluer::Result<()> {
        self.read_char = None;
        self.write_char = None;
        self.device.disconnect().await
    }
}

async fn find_characteristics(
    device: &Device,
) -> bluer::Result<Option<(Characteristic, Characteristic)>> {
    let mut read_char = None;
    let mut write_char = None;

    for service in device.services().await? {
        let uuid = service.uuid().await?;
        if uuid == SERVICE_UUID {
            for char in service.characteristics().await? {
                let uuid = char.uuid().await?;
                if uuid == READ_CHAR_UUID {
                    read_char = Some(char);
                } else if uuid == WRITE_CHAR_UUID {
                    write_char = Some(char);
                }
            }
        }
    }

    if let Some(read_char) = read_char {
        if let Some(write_char) = write_char {
            return Ok(Some((read_char, write_char)));
        }
    }
    Ok(None)
}

fn gen_cmd(cmd: u8, payload_length: usize) -> Vec<u8> {
    let mut data = vec![0u8; 3 + payload_length];
    data[0] = 0x57;
    data[1] = if cmd > 0x0f { 0x0f } else { 0 };
    data[2] = cmd;
    data
}

mod cli {
    use clap::Parser;
    use std::str::FromStr;

    #[derive(Debug, Parser)]
    pub struct Args {
        #[clap(long, value_parser)]
        pub discover: bool,

        #[clap(long, short, value_parser)]
        pub dump_historic: bool,

        #[clap(long, value_parser=parse_duration)]
        pub dump_last: Option<chrono::Duration>,

        #[clap(long, value_parser)]
        pub set_time: bool,

        #[clap(value_parser=parse_addr)]
        pub address: Option<bluer::Address>,
    }

    fn parse_addr(s: &str) -> Result<bluer::Address, &'static str> {
        bluer::Address::from_str(s).map_err(|_| "invalid address")
    }

    fn parse_duration(s: &str) -> Result<chrono::Duration, &'static str> {
        let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
        let mut value = digits.parse::<i64>().map_err(|_| "invalid number")?;
        let unit: String = s.chars().skip(digits.len()).collect();
        value *= match unit.as_str() {
            "m" => 1,
            "h" => 60,
            "d" => 60 * 24,
            _ => return Err("invalid time unit"),
        };

        Ok(chrono::Duration::minutes(value))
    }

    #[cfg(test)]
    mod tests {
        use crate::cli::parse_duration;

        #[test]
        fn parses_durations() {
            assert_eq!(parse_duration("1d"), Ok(chrono::Duration::days(1)));
            assert_eq!(parse_duration("5m"), Ok(chrono::Duration::minutes(5)));
            assert_eq!(parse_duration("42h"), Ok(chrono::Duration::hours(42)));
        }
    }
}

fn dump_csv(index_info: &MeterSectionInfo, samples: &[MeterSampleValue]) {
    assert!(samples.len() <= u16::MAX.into());
    let samples_len: u16 = samples.len().try_into().unwrap();

    let interval = Duration::seconds(index_info.interval.into());
    let mut current_time = Local.timestamp(index_info.start_time.into(), 0)
        + (interval * (index_info.data_length - samples_len).into());

    for value in samples {
        println!(
            "{}\t{}\t{}",
            current_time, value.temperature, value.humidity
        );
        current_time = current_time + interval;
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let args = cli::Args::parse();

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let started = Instant::now();
    let discover = adapter.discover_devices().await?;
    pin_mut!(discover);
    while let Some(evt) = discover.next().await {
        if let AdapterEvent::DeviceAdded(addr) = evt {
            if let Some(wanted_addr) = args.address {
                if addr != wanted_addr {
                    continue;
                }
            }

            let device = adapter.device(addr)?;
            if let Some(service_data) = device.service_data().await? {
                if let Some(data) = service_data.get(&ADVERTISEMENT_SERVICE_UUID) {
                    if args.set_time {
                        let mut meter = Meter::new(&adapter, addr)?;
                        meter.set_time().await?;
                        meter.disconnect().await?;
                    }

                    if args.dump_historic || args.dump_last.is_some() {
                        let mut meter = Meter::new(&adapter, addr)?;
                        if let Some(index_info) = meter.read_section_info().await? {
                            let samples = meter.read_samples(&index_info, args.dump_last).await?;
                            dump_csv(&index_info, &samples);
                        }
                        meter.disconnect().await?;
                    } else if let Some(value) = MeterValue::from_data(data) {
                        println!(
                            "{}: {}°C, {}% humidity, {}% battery",
                            addr, value.temperature, value.humidity, value.battery
                        );
                    }
                }
            }

            if args.address.is_some() {
                break;
            }
        }

        if Instant::now() - started > std::time::Duration::new(10, 0) {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{MeterSampleValue, MeterSectionInfo, MeterValue};

    #[test]
    fn parses_service_data() {
        let service_data = vec![105, 0, 228, 9, 152, 40];
        let result = MeterValue::from_data(&service_data);
        assert_eq!(
            result,
            Some(MeterValue {
                temperature: 24.9,
                humidity: 40,
                battery: 100
            })
        );
    }

    #[test]
    fn parses_sample_info() {
        let response = vec![1, 152, 40, 119, 152, 40, 152, 40, 120, 152, 40];
        let result = MeterSampleValue::from_response(&response);
        assert_eq!(
            result,
            Some(
                vec![(24.7, 40), (24.7, 40), (24.7, 40), (24.8, 40)]
                    .into_iter()
                    .map(|(temperature, humidity)| MeterSampleValue {
                        temperature,
                        humidity
                    })
                    .collect()
            )
        );
    }

    #[test]
    fn parse_section_info() {
        let response = vec![1, 97, 160, 191, 231, 97, 162, 162, 63, 4, 6, 0, 120];
        let result = MeterSectionInfo::from_response(&response);
        assert_eq!(
            result,
            Some(MeterSectionInfo {
                start_time: 1_637_924_839,
                end_time: 1_638_048_319,
                interval: 120,
                data_length: 1030
            })
        );
    }
}
