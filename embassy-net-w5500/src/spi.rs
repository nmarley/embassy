use embedded_hal_async::spi::{Operation, SpiDevice};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SpiInterface<SPI>(pub SPI);

impl<SPI: SpiDevice> SpiInterface<SPI> {
    pub async fn read_frame(&mut self, address: u16, data: &mut [u8]) -> Result<(), SPI::Error> {
        self.0
            .transaction(&mut [
                Operation::Write(&[0x0F, (address >> 8) as u8, address as u8]),
                Operation::Read(data),
            ])
            .await
    }

    pub async fn write_frame(&mut self, address: u16, data: &[u8]) -> Result<(), SPI::Error> {
        self.0
            .transaction(&mut [
                Operation::Write(&[0xF0, (address >> 8) as u8, address as u8]),
                Operation::Write(data),
            ])
            .await
    }
}
