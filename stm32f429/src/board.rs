use embassy_stm32::peripherals::RNG;
use embassy_stm32::rng::Rng;

// Separating the board from the network init task
pub struct Board {
    // Pins for ethernet
    pub peri: embassy_stm32::peripherals::ETH,
    pub ref_clk: embassy_stm32::peripherals::PA1,
    // management data input output between PHY and MAC layers
    pub mdio: embassy_stm32::peripherals::PA2,
    // management data clock, for sync between PHY and MAC
    pub mdc: embassy_stm32::peripherals::PC1,
    // carrier sense, sensing if data is transmitted
    pub crs: embassy_stm32::peripherals::PA7,
    pub rx_d0: embassy_stm32::peripherals::PC4,
    pub rx_d1: embassy_stm32::peripherals::PC5,
    pub tx_d0: embassy_stm32::peripherals::PG13,
    pub tx_d1: embassy_stm32::peripherals::PB13,
    // transmit enable
    pub tx_en: embassy_stm32::peripherals::PG11,
    // our random souce
    pub rng: embassy_stm32::rng::Rng<'static, RNG>,
}

impl Board {
    pub fn new(p: embassy_stm32::Peripherals) -> Self {
        Self {
            peri: p.ETH,
            ref_clk: p.PA1,
            mdio: p.PA2,
            mdc: p.PC1,
            crs: p.PA7,
            rx_d0: p.PC4,
            rx_d1: p.PC5,
            tx_d0: p.PG13,
            tx_d1: p.PB13,
            tx_en: p.PG11,
            rng: Rng::new(p.RNG, crate::Irqs),
        }
    }
}
