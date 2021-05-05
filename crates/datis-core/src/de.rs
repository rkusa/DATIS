use serde::Deserialize;
use uom::si::angle::radian;
use uom::si::f64::{Angle, Pressure, ThermodynamicTemperature as Temperature, Velocity};
use uom::si::i32::Length;
use uom::si::length::meter;
use uom::si::pressure::pascal;
use uom::si::thermodynamic_temperature::degree_celsius;
use uom::si::velocity::meter_per_second;

#[allow(unused)]
pub fn from_meter_per_second<'de, D>(deserializer: D) -> Result<Velocity, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let v = f64::deserialize(deserializer)?;
    Ok(Velocity::new::<meter_per_second>(v))
}

#[allow(unused)]
pub fn from_radian<'de, D>(deserializer: D) -> Result<Angle, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let v = f64::deserialize(deserializer)?;
    Ok(Angle::new::<radian>(v))
}

#[allow(unused)]
pub fn from_pascal<'de, D>(deserializer: D) -> Result<Pressure, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let v = f64::deserialize(deserializer)?;
    Ok(Pressure::new::<pascal>(v))
}

#[allow(unused)]
pub fn from_meter<'de, D>(deserializer: D) -> Result<Length, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let v = f64::deserialize(deserializer)?;
    Ok(Length::new::<meter>(v.round() as i32))
}

#[allow(unused)]
pub fn from_degree_celcius<'de, D>(deserializer: D) -> Result<Temperature, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let v = f64::deserialize(deserializer)?;
    Ok(Temperature::new::<degree_celsius>(v))
}
