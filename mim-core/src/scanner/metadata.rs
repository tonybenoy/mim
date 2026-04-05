use crate::models::Photo;
use crate::Error;
use std::path::Path;

pub fn extract_exif(path: &Path) -> std::result::Result<Vec<(String, String)>, Error> {
    let file = std::fs::File::open(path)?;
    let mut bufreader = std::io::BufReader::new(&file);
    let exif = exif::Reader::new()
        .read_from_container(&mut bufreader)
        .map_err(|e| Error::Exif(e.to_string()))?;

    let mut fields = Vec::new();
    for f in exif.fields() {
        let tag = f.tag.to_string();
        let value = f.display_value().with_unit(&exif).to_string();
        fields.push((tag, value));
    }
    Ok(fields)
}

pub fn apply_exif(path: &Path, photo: &mut Photo) -> std::result::Result<(), Error> {
    let file = std::fs::File::open(path)?;
    let mut bufreader = std::io::BufReader::new(&file);
    let exif = match exif::Reader::new().read_from_container(&mut bufreader) {
        Ok(exif) => exif,
        Err(_) => return Ok(()), // No EXIF is fine
    };

    // Date taken
    if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
        if let exif::Value::Ascii(ref vec) = field.value {
            if let Some(bytes) = vec.first() {
                if let Ok(s) = std::str::from_utf8(bytes) {
                    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S") {
                        photo.taken_at = Some(dt.and_utc());
                    }
                }
            }
        }
    }

    // Camera info
    if let Some(field) = exif.get_field(exif::Tag::Make, exif::In::PRIMARY) {
        photo.camera_make = Some(field.display_value().to_string().trim_matches('"').to_string());
    }
    if let Some(field) = exif.get_field(exif::Tag::Model, exif::In::PRIMARY) {
        photo.camera_model = Some(field.display_value().to_string().trim_matches('"').to_string());
    }
    if let Some(field) = exif.get_field(exif::Tag::LensModel, exif::In::PRIMARY) {
        photo.lens_model = Some(field.display_value().to_string().trim_matches('"').to_string());
    }

    // Exposure settings
    if let Some(field) = exif.get_field(exif::Tag::FocalLength, exif::In::PRIMARY) {
        if let exif::Value::Rational(ref v) = field.value {
            if let Some(r) = v.first() {
                photo.focal_length = Some(r.num as f64 / r.denom as f64);
            }
        }
    }
    if let Some(field) = exif.get_field(exif::Tag::FNumber, exif::In::PRIMARY) {
        if let exif::Value::Rational(ref v) = field.value {
            if let Some(r) = v.first() {
                photo.aperture = Some(r.num as f64 / r.denom as f64);
            }
        }
    }
    if let Some(field) = exif.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY) {
        photo.shutter_speed = Some(field.display_value().to_string());
    }
    if let Some(field) = exif.get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY) {
        if let exif::Value::Short(ref v) = field.value {
            if let Some(&iso) = v.first() {
                photo.iso = Some(iso as u32);
            }
        }
    }

    // GPS
    let lat = extract_gps_coord(&exif, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef);
    let lng = extract_gps_coord(&exif, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef);
    photo.latitude = lat;
    photo.longitude = lng;

    if let Some(field) = exif.get_field(exif::Tag::GPSAltitude, exif::In::PRIMARY) {
        if let exif::Value::Rational(ref v) = field.value {
            if let Some(r) = v.first() {
                photo.altitude = Some(r.num as f64 / r.denom as f64);
            }
        }
    }

    Ok(())
}

fn extract_gps_coord(exif: &exif::Exif, coord_tag: exif::Tag, ref_tag: exif::Tag) -> Option<f64> {
    let field = exif.get_field(coord_tag, exif::In::PRIMARY)?;
    if let exif::Value::Rational(ref v) = field.value {
        if v.len() >= 3 {
            let degrees = v[0].num as f64 / v[0].denom as f64;
            let minutes = v[1].num as f64 / v[1].denom as f64;
            let seconds = v[2].num as f64 / v[2].denom as f64;
            let mut coord = degrees + minutes / 60.0 + seconds / 3600.0;

            if let Some(ref_field) = exif.get_field(ref_tag, exif::In::PRIMARY) {
                if let exif::Value::Ascii(ref refs) = ref_field.value {
                    if let Some(r) = refs.first() {
                        if r == b"S" || r == b"W" {
                            coord = -coord;
                        }
                    }
                }
            }
            return Some(coord);
        }
    }
    None
}
