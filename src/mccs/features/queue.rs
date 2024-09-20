use crate::ddc::{DdcCiDevice, DdcError};

use super::{NewControlValue, VcpFeatureCode, VcpFeatureValue};

pub struct VcpCodeUpdateQueue<'ddc, D: DdcCiDevice> {
    ddc_channel: &'ddc mut D,
    started: bool
}

impl<'ddc, D: DdcCiDevice> VcpCodeUpdateQueue<'ddc, D> {
    pub fn new(ddc_channel: &'ddc mut D) -> Self {
        // initialize feature update readout
        Self { ddc_channel, started: false }
    }
}

impl<'ddc, D: DdcCiDevice> Iterator for VcpCodeUpdateQueue<'ddc, D> {
    type Item = Result<VcpFeatureValue, DdcError>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.started {
            let r = self.ddc_channel.get_vcp_feature::<NewControlValue>();
            if r.is_err() || r.as_ref().is_ok_and(|cv| *cv != NewControlValue::NewControlValuesPresent) {
                eprintln!("no new control values");
                return None;
            } else {
                eprintln!("started reading fifo");
                self.started = true;
            }
        }
        eprintln!("read value");
        let result = self.ddc_channel.get_vcp_feature::<VcpFeatureCode>();
        if result.as_ref().is_ok_and(|feature| *feature == VcpFeatureCode::CodePage) {
            let _ = self.ddc_channel.set_vcp_feature(NewControlValue::Finished);
            return None;
        }
        let result = result.map(|feature| {
            match feature {
                VcpFeatureCode::Luminance
                | VcpFeatureCode::Contrast
                | VcpFeatureCode::OsdLanguage
                | VcpFeatureCode::InputSelect => {
                    VcpFeatureValue::read_from_ddc(self.ddc_channel, feature).unwrap()
                }
                _ => {
                    VcpFeatureValue::Unimplemented(feature.into(), 0)
                }
            }
        });
        Some(result)
    }
}
