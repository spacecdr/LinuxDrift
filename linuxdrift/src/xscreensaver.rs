//! XScreenSaver supplies an X11 parent even when the host desktop is Wayland.
use crate::config::Result;
use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{ChangeWindowAttributesAux, ConnectionExt, EventMask},
        Event,
    },
    rust_connection::RustConnection,
};

pub struct Parent {
    connection: RustConnection,
    pub id: u32,
    pub width: u32,
    pub height: u32,
}
impl Parent {
    pub fn open(id: Option<&str>) -> Result<Self> {
        let (connection, screen) = x11rb::connect(None)?;
        let id = match id {
            Some(value) => parse_id(value)?,
            None => connection.setup().roots[screen].root,
        };
        let geometry = connection.get_geometry(id)?.reply()?;
        connection
            .change_window_attributes(
                id,
                &ChangeWindowAttributesAux::new().event_mask(EventMask::STRUCTURE_NOTIFY),
            )?
            .check()?;
        Ok(Self {
            connection,
            id,
            width: geometry.width.into(),
            height: geometry.height.into(),
        })
    }
    /// Some(false): parent destroyed; Some(true): size changed.
    pub fn poll(&mut self) -> Result<Option<bool>> {
        let mut changed = None;
        while let Some(event) = self.connection.poll_for_event()? {
            match event {
                Event::ConfigureNotify(event) if event.window == self.id => {
                    self.width = event.width.into();
                    self.height = event.height.into();
                    changed = Some(true);
                }
                Event::DestroyNotify(event) if event.window == self.id => return Ok(Some(false)),
                _ => (),
            }
        }
        Ok(changed)
    }
}
fn parse_id(value: &str) -> Result<u32> {
    let value = value.trim();
    let id = if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u32::from_str_radix(hex, 16)?
    } else {
        value.parse()?
    };
    if id == 0 {
        return Err("X11 window ID cannot be zero".into());
    }
    Ok(id)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_xscreensaver_ids() {
        assert_eq!(parse_id("0x100").unwrap(), 256);
        assert_eq!(parse_id("256").unwrap(), 256);
        assert!(parse_id("0").is_err());
        assert!(parse_id("bad").is_err());
    }
}
