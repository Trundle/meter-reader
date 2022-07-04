const RESPONSE_OK: u8 = 1;

#[derive(Debug, Eq, PartialEq)]
pub struct MeterSectionInfo {
    pub start_time: u32,
    pub end_time: u32,
    pub data_length: u16,
    pub interval: u16,
}

impl MeterSectionInfo {
    #![allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn from_response(data: &[u8]) -> Option<MeterSectionInfo> {
        if data.len() < 13 || data[0] != RESPONSE_OK {
            return None;
        }

        let start_time = u32::from_be_bytes(data[1..5].try_into().unwrap());
        let end_time = u32::from_be_bytes(data[5..9].try_into().unwrap());
        let data_length = u16::from_be_bytes(data[9..11].try_into().unwrap());
        let interval = u16::from_be_bytes(data[11..13].try_into().unwrap());

        Some(MeterSectionInfo {
            start_time,
            end_time,
            data_length,
            interval,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct MeterSampleValue {
    pub temperature: f32,
    pub humidity: u8,
}

impl MeterSampleValue {
    #[must_use]
    pub fn from_response(data: &[u8]) -> Option<Vec<MeterSampleValue>> {
        if data.len() < 6 || data[0] != RESPONSE_OK || (data.len() - 1) % 5 != 0 {
            return None;
        }

        let mut result = Vec::with_capacity((data.len() - 1) / 6);

        for i in (1..(data.len() - 1)).step_by(5) {
            result.push(MeterSampleValue::first_value(&data[i..]));
            result.push(MeterSampleValue::second_value(&data[i..]));
        }

        Some(result)
    }

    fn first_value(data: &[u8]) -> MeterSampleValue {
        assert!(data.len() >= 3);

        let mut temperature = f32::from(data[0] & 0x7f) + (f32::from((data[2] >> 4) & 0xf) / 10.0);
        if (data[0] & 0x80) == 0 {
            temperature = -temperature;
        }

        let humidity = data[1] & 0x7f;

        MeterSampleValue {
            temperature,
            humidity,
        }
    }

    fn second_value(data: &[u8]) -> MeterSampleValue {
        assert!(data.len() >= 5);

        let mut temperature = f32::from(data[3] & 0x7f) + (f32::from(data[2] & 0xf) / 10.0);
        if (data[3] & 0x80) == 0 {
            temperature = -temperature;
        }

        let humidity = data[4] & 0x7f;

        MeterSampleValue {
            temperature,
            humidity,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct MeterValue {
    pub temperature: f32,
    pub humidity: u8,
    pub battery: u8,
}

impl MeterValue {
    #[must_use]
    pub fn from_data(data: &[u8]) -> Option<MeterValue> {
        if data.len() != 6 || data[0] != 105 {
            return None;
        }

        let mut temperature = f32::from(data[4] & 0x7f) + (f32::from(data[3] & 0xf) / 10.0);
        if (data[4] & 0x80) == 0 {
            temperature = -temperature;
        }

        let humidity = data[5] & 0x7f;
        let battery = data[2] & 0x7f;

        Some(MeterValue {
            temperature,
            humidity,
            battery,
        })
    }
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
