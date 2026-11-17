// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! CSS length conversion and WCAG contrast.

use css_color_parser2::Color;

pub const UNITS: [&str; 10] = ["px", "rem", "em", "pt", "pc", "in", "cm", "mm", "vw", "vh"];

/// The context relative units are resolved against.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub root_px: f64,
    pub parent_px: f64,
    pub viewport_width_px: f64,
    pub viewport_height_px: f64,
}

/// Pixels per one unit. CSS fixes 1in = 96px.
fn pixels_per(unit: &str, context: Context) -> Option<f64> {
    Some(match unit {
        "px" => 1.0,
        "rem" => context.root_px,
        "em" => context.parent_px,
        "pt" => 96.0 / 72.0,
        "pc" => 16.0,
        "in" => 96.0,
        "cm" => 96.0 / 2.54,
        "mm" => 96.0 / 25.4,
        "vw" => context.viewport_width_px / 100.0,
        "vh" => context.viewport_height_px / 100.0,
        _ => return None,
    })
}

fn format_number(value: f64) -> String {
    let text = format!("{value:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The value expressed in every unit.
pub fn convert_all(
    value: f64,
    from: &str,
    context: Context,
) -> Result<Vec<(String, String)>, String> {
    let from_px = pixels_per(from, context).ok_or_else(|| format!("Unknown unit {from}"))?;
    if [
        context.root_px,
        context.parent_px,
        context.viewport_width_px,
        context.viewport_height_px,
    ]
    .iter()
    .any(|v| !v.is_finite() || *v <= 0.0)
    {
        return Err("Root size, parent size and viewport must be positive numbers".to_string());
    }
    let pixels = value * from_px;
    Ok(UNITS
        .iter()
        .map(|unit| {
            let per = pixels_per(unit, context).unwrap_or(1.0);
            (
                unit.to_string(),
                format!("{}{unit}", format_number(pixels / per)),
            )
        })
        .collect())
}

pub fn parse_color(text: &str) -> Result<Color, String> {
    text.trim()
        .parse::<Color>()
        .map_err(|_| format!("\"{}\" is not a CSS colour", text.trim()))
}

fn channel(value: u8) -> f64 {
    let c = f64::from(value) / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Relative luminance as defined by WCAG 2.x.
pub fn luminance(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

pub fn contrast_ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let (la, lb) = (luminance(a.0, a.1, a.2), luminance(b.0, b.1, b.2));
    let (lighter, darker) = if la >= lb { (la, lb) } else { (lb, la) };
    (lighter + 0.05) / (darker + 0.05)
}

/// Blend a translucent colour onto an opaque backdrop.
fn composite(top: &Color, backdrop: (u8, u8, u8)) -> (u8, u8, u8) {
    let alpha = top.a.clamp(0.0, 1.0);
    let mix = |t: u8, b: u8| (f32::from(t) * alpha + f32::from(b) * (1.0 - alpha)).round() as u8;
    (
        mix(top.r, backdrop.0),
        mix(top.g, backdrop.1),
        mix(top.b, backdrop.2),
    )
}

pub struct Contrast {
    pub ratio: f64,
    pub foreground: (u8, u8, u8),
    pub background: (u8, u8, u8),
    pub rows: Vec<(String, String)>,
}

/// Contrast of two CSS colours. A translucent background is shown on white, a translucent foreground on the background.
pub fn contrast(foreground: &str, background: &str) -> Result<Contrast, String> {
    let fg = parse_color(foreground)?;
    let bg = parse_color(background)?;
    let background = composite(&bg, (255, 255, 255));
    let foreground = composite(&fg, background);
    let ratio = contrast_ratio(foreground, background);

    let verdict = |threshold: f64| {
        if ratio >= threshold {
            "✓ pass"
        } else {
            "✗ fail"
        }
        .to_string()
    };
    let rows = vec![
        (
            "Contrast ratio".to_string(),
            format!("{:.2} : 1", (ratio * 100.0).floor() / 100.0),
        ),
        ("AA, normal text (4.5)".to_string(), verdict(4.5)),
        ("AA, large text (3)".to_string(), verdict(3.0)),
        ("AAA, normal text (7)".to_string(), verdict(7.0)),
        ("AAA, large text (4.5)".to_string(), verdict(4.5)),
        ("UI components (3)".to_string(), verdict(3.0)),
    ];
    Ok(Contrast {
        ratio,
        foreground,
        background,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTEXT: Context = Context {
        root_px: 16.0,
        parent_px: 20.0,
        viewport_width_px: 1000.0,
        viewport_height_px: 500.0,
    };

    fn get<'a>(rows: &'a [(String, String)], label: &str) -> &'a str {
        rows.iter()
            .find(|(l, _)| l == label)
            .map(|(_, v)| v.as_str())
            .unwrap()
    }

    #[test]
    fn converts_between_units() {
        let rows = convert_all(1.0, "in", CONTEXT).unwrap();
        assert_eq!(get(&rows, "px"), "96px");
        assert_eq!(get(&rows, "pt"), "72pt");
        assert_eq!(get(&rows, "pc"), "6pc");
        assert_eq!(get(&rows, "cm"), "2.54cm");
        assert_eq!(get(&rows, "mm"), "25.4mm");
        assert_eq!(get(&rows, "rem"), "6rem");

        let rows = convert_all(2.0, "rem", CONTEXT).unwrap();
        assert_eq!(get(&rows, "px"), "32px");
        assert_eq!(get(&rows, "em"), "1.6em");

        let rows = convert_all(10.0, "vw", CONTEXT).unwrap();
        assert_eq!(get(&rows, "px"), "100px");
        assert_eq!(get(&rows, "vh"), "20vh");
        assert!(convert_all(1.0, "furlong", CONTEXT).is_err());
        assert!(
            convert_all(
                1.0,
                "px",
                Context {
                    root_px: 0.0,
                    ..CONTEXT
                }
            )
            .is_err()
        );
    }

    #[test]
    fn wcag_reference_ratios() {
        assert!((contrast_ratio((0, 0, 0), (255, 255, 255)) - 21.0).abs() < 1e-9);
        assert!((contrast_ratio((255, 255, 255), (255, 255, 255)) - 1.0).abs() < 1e-9);
        // #767676 on white is the well-known lightest grey that still passes AA for normal text.
        let grey = contrast_ratio((0x76, 0x76, 0x76), (255, 255, 255));
        assert!((4.5..4.6).contains(&grey), "{grey}");
        let almost = contrast_ratio((0x77, 0x77, 0x77), (255, 255, 255));
        assert!(almost < 4.5, "{almost}");
    }

    #[test]
    fn contrast_verdicts_and_colour_formats() {
        let result = contrast("#767676", "white").unwrap();
        assert_eq!(get(&result.rows, "Contrast ratio"), "4.54 : 1");
        assert_eq!(get(&result.rows, "AA, normal text (4.5)"), "✓ pass");
        assert_eq!(get(&result.rows, "AAA, normal text (7)"), "✗ fail");
        assert_eq!(get(&result.rows, "AAA, large text (4.5)"), "✓ pass");

        let same = contrast("rgb(10, 20, 30)", "#0a141e").unwrap();
        assert_eq!(get(&same.rows, "Contrast ratio"), "1.00 : 1");

        // 50 % black over white is #808080.
        let translucent = contrast("rgba(0, 0, 0, 0.5)", "#fff").unwrap();
        assert_eq!(translucent.foreground, (128, 128, 128));
        assert!(contrast("nonsense", "#fff").is_err());
    }
}
