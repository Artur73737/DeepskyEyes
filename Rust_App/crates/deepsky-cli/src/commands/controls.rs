//! Additional explicit controls, never silently translated into other controls.
use deepsky_camera::model::{CaptureSettings, CropRect, JpegSettings, JpegSize, WhiteBalanceRequest};

fn parse_jpeg_size(value: &str) -> Result<JpegSize, String> {
    let (w, h) = value.split_once(['x', 'X']).ok_or("--jpeg-thumbnail-size requires WIDTHxHEIGHT".to_string())?;
    let (width, height) = (w.parse().map_err(|_| "--jpeg-thumbnail-size requires WIDTHxHEIGHT".to_string())?, h.parse().map_err(|_| "--jpeg-thumbnail-size requires WIDTHxHEIGHT".to_string())?);
    Ok(JpegSize { width, height })
}

pub fn parse(args: &[String]) -> Result<CaptureSettings, String> {
    let mut settings = CaptureSettings::default();
    settings.frame_duration_ns = crate::flag(args, "frame-duration-ns")
        .map(|v| v.parse().map_err(|_| "invalid --frame-duration-ns".to_string())).transpose()?;
    settings.ois = crate::flag(args, "ois");
    settings.eis = crate::flag(args, "eis");
    if let Some(v) = crate::flag(args, "crop") {
        let values: Vec<u32> = v.split(',').map(str::parse).collect::<Result<_,_>>()
            .map_err(|_| "--crop requires x,y,width,height".to_string())?;
        let [x,y,width,height] = values.as_slice() else { return Err("--crop requires x,y,width,height".into()); };
        settings.crop = Some(CropRect { x:*x,y:*y,width:*width,height:*height });
    }
    if let Some(v) = crate::flag(args, "processing") {
        for entry in v.split(',') {
            let (key,value) = entry.split_once('=').ok_or("--processing requires key=mode,key=mode")?;
            if key.is_empty() || value.is_empty() || settings.processing.insert(key.into(),value.into()).is_some() {
                return Err("empty or duplicate processing control".into());
            }
        }
    }
    if let Some(v) = crate::flag(args, "wb-kelvin") {
        if crate::flag(args,"wb-preset").is_some() { return Err("--wb-kelvin and --wb-preset are exclusive".into()); }
        settings.white_balance = Some(WhiteBalanceRequest::Temperature {
            kelvin: v.parse().map_err(|_| "invalid --wb-kelvin")?, tint: None
        });
    }
    // Typed JPEG output controls. Any one of them activates the block; device
    // ranges (quality 1..100, announced thumbnail sizes, JPEG stream) are
    // enforced downstream with Reject, never clamped here.
    let jpeg_quality = crate::flag(args, "jpeg-quality").map(|v| v.parse::<u8>().map_err(|_| "invalid --jpeg-quality (1..100)".to_string())).transpose()?;
    let jpeg_orientation = crate::flag(args, "jpeg-orientation").map(|v| v.parse::<u16>().map_err(|_| "invalid --jpeg-orientation (0|90|180|270)".to_string())).transpose()?;
    let jpeg_thumbnail_quality = crate::flag(args, "jpeg-thumbnail-quality").map(|v| v.parse::<u8>().map_err(|_| "invalid --jpeg-thumbnail-quality (1..100)".to_string())).transpose()?;
    let jpeg_thumbnail_size = crate::flag(args, "jpeg-thumbnail-size").map(|v| parse_jpeg_size(&v)).transpose()?;
    if jpeg_quality.is_some() || jpeg_orientation.is_some() || jpeg_thumbnail_quality.is_some() || jpeg_thumbnail_size.is_some() {
        settings.jpeg = Some(JpegSettings { quality: jpeg_quality, orientation: jpeg_orientation, thumbnail_quality: jpeg_thumbnail_quality, thumbnail_size: jpeg_thumbnail_size });
    }
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::parse;
    fn args(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }
    #[test] fn controls_are_preserved() {
        let s = parse(&args(&["--crop","1,2,30,40","--ois","on","--eis","off","--frame-duration-ns","123456","--processing","edge=fast,shading=off"])).unwrap();
        assert_eq!(s.crop.unwrap().width,30); assert_eq!(s.frame_duration_ns,Some(123456));
        assert_eq!(s.processing["edge"],"fast"); assert_eq!(s.ois.as_deref(),Some("on"));
    }
    #[test] fn malformed_controls_fail() {
        for v in [vec!["--crop","1,2,3"],vec!["--processing","edge=off,edge=fast"],vec!["--wb-kelvin","5000","--wb-preset","daylight"]] {
            assert!(parse(&args(&v)).is_err());
        }
    }
    #[test] fn jpeg_controls_parsed() {
        let s = parse(&args(&["--jpeg-quality","90","--jpeg-orientation","90","--jpeg-thumbnail-size","320x240"])).unwrap();
        let jpeg = s.jpeg.unwrap();
        assert_eq!(jpeg.quality, Some(90)); assert_eq!(jpeg.orientation, Some(90));
        assert_eq!(jpeg.thumbnail_size.unwrap().width, 320);
        assert!(parse(&args(&[])).unwrap().jpeg.is_none());
    }
    #[test] fn malformed_jpeg_controls_fail() {
        for v in [vec!["--jpeg-quality","300"],vec!["--jpeg-orientation","45x"],vec!["--jpeg-thumbnail-size","320"]] {
            assert!(parse(&args(&v)).is_err(), "{v:?}");
        }
    }
}
