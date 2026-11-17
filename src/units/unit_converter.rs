// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

pub struct UnitDef {
    pub name: &'static str,
    pub symbol: &'static str,
    /// Conversion factor to the category's base unit.
    /// Not used for Temperature — see `convert_temperature`.
    pub(crate) factor: f64,
}

impl UnitDef {
    const fn new(name: &'static str, symbol: &'static str, factor: f64) -> Self {
        Self { name, symbol, factor }
    }
}

static LENGTH_UNITS: &[UnitDef] = &[
    UnitDef::new("Millimeter", "mm", 0.001),
    UnitDef::new("Centimeter", "cm", 0.01),
    UnitDef::new("Meter", "m", 1.0),
    UnitDef::new("Kilometer", "km", 1_000.0),
    UnitDef::new("Inch", "in", 0.0254),
    UnitDef::new("Foot", "ft", 0.3048),
    UnitDef::new("Yard", "yd", 0.9144),
    UnitDef::new("Mile", "mi", 1_609.344),
    UnitDef::new("Nautical Mile", "nmi", 1_852.0),
];

static MASS_UNITS: &[UnitDef] = &[
    UnitDef::new("Milligram", "mg", 0.000_001),
    UnitDef::new("Gram", "g", 0.001),
    UnitDef::new("Kilogram", "kg", 1.0),
    UnitDef::new("Tonne", "t", 1_000.0),
    UnitDef::new("Ounce", "oz", 0.028_349_523),
    UnitDef::new("Pound", "lb", 0.453_592_37),
    UnitDef::new("Stone", "st", 6.350_293_18),
];

static TEMPERATURE_UNITS: &[UnitDef] = &[
    UnitDef::new("Celsius", "°C", 1.0),
    UnitDef::new("Fahrenheit", "°F", 1.0),
    UnitDef::new("Kelvin", "K", 1.0),
];

static SPEED_UNITS: &[UnitDef] = &[
    UnitDef::new("Meter per second", "m/s", 1.0),
    UnitDef::new("Kilometer per hour", "km/h", 0.277_777_778),
    UnitDef::new("Mile per hour", "mph", 0.447_04),
    UnitDef::new("Knot", "kn", 0.514_444),
    UnitDef::new("Foot per second", "ft/s", 0.3048),
];

static AREA_UNITS: &[UnitDef] = &[
    UnitDef::new("Square millimeter", "mm²", 0.000_001),
    UnitDef::new("Square centimeter", "cm²", 0.000_1),
    UnitDef::new("Square meter", "m²", 1.0),
    UnitDef::new("Square kilometer", "km²", 1_000_000.0),
    UnitDef::new("Square inch", "in²", 0.000_645_16),
    UnitDef::new("Square foot", "ft²", 0.092_903_04),
    UnitDef::new("Square yard", "yd²", 0.836_127_36),
    UnitDef::new("Acre", "ac", 4_046.856_422_4),
    UnitDef::new("Hectare", "ha", 10_000.0),
    UnitDef::new("Square mile", "mi²", 2_589_988.110_336),
];


static VOLUME_UNITS: &[UnitDef] = &[
    UnitDef::new("Milliliter", "ml", 0.001),
    UnitDef::new("Liter", "l", 1.0),
    UnitDef::new("Cubic meter", "m³", 1_000.0),
    UnitDef::new("Cubic centimeter", "cm³", 0.001),
    UnitDef::new("Fluid ounce (US)", "fl oz", 0.029_573_529_6),
    UnitDef::new("Cup (US)", "cup", 0.236_588_236_5),
    UnitDef::new("Pint (US)", "pt", 0.473_176_473),
    UnitDef::new("Quart (US)", "qt", 0.946_352_946),
    UnitDef::new("Gallon (US)", "gal", 3.785_411_784),
    UnitDef::new("Tablespoon (US)", "tbsp", 0.014_786_764_78),
    UnitDef::new("Teaspoon (US)", "tsp", 0.004_928_921_59),
];


static TIME_UNITS: &[UnitDef] = &[
    UnitDef::new("Millisecond", "ms", 0.001),
    UnitDef::new("Second", "s", 1.0),
    UnitDef::new("Minute", "min", 60.0),
    UnitDef::new("Hour", "h", 3_600.0),
    UnitDef::new("Day", "d", 86_400.0),
    UnitDef::new("Week", "wk", 604_800.0),
    UnitDef::new("Month (avg)", "mo", 2_629_800.0),
    UnitDef::new("Year", "yr", 31_557_600.0),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Length,
    Mass,
    Temperature,
    Speed,
    Area,
    Volume,
    Time,
}

impl Category {
    pub const ALL: &'static [Category] = &[
        Category::Length,
        Category::Mass,
        Category::Temperature,
        Category::Speed,
        Category::Area,
        Category::Volume,
        Category::Time,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Category::Length => "Length",
            Category::Mass => "Mass",
            Category::Temperature => "Temperature",
            Category::Speed => "Speed",
            Category::Area => "Area",
            Category::Volume => "Volume",
            Category::Time => "Time",
        }
    }

    pub fn units(self) -> &'static [UnitDef] {
        match self {
            Category::Length => LENGTH_UNITS,
            Category::Mass => MASS_UNITS,
            Category::Temperature => TEMPERATURE_UNITS,
            Category::Speed => SPEED_UNITS,
            Category::Area => AREA_UNITS,
            Category::Volume => VOLUME_UNITS,
            Category::Time => TIME_UNITS,
        }
    }
    
    pub fn convert(self, value: f64, from_index: usize, to_index: usize) -> Option<f64> {
        if self == Category::Temperature {
            return convert_temperature(value, from_index, to_index);
        }

        let units = self.units();
        let from = units.get(from_index)?;
        let to = units.get(to_index)?;

        Some(value * from.factor / to.factor)
    }
}

fn convert_temperature(value: f64, from_index: usize, to_index: usize) -> Option<f64> {
    let celsius = match from_index {
        0 => value,
        1 => (value - 32.0) * 5.0 / 9.0,
        2 => value - 273.15,
        _ => return None,
    };

    let result = match to_index {
        0 => celsius,
        1 => celsius * 9.0 / 5.0 + 32.0,
        2 => celsius + 273.15,
        _ => return None,
    };

    Some(result)
}
