//! Additional explicit controls, never silently translated into other controls.
use deepsky_camera::model::{CaptureSettings, CropRect, WhiteBalanceRequest};

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
}
