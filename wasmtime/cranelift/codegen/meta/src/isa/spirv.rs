use crate::cdsl::{isa::TargetIsa, settings::SettingGroupBuilder};

pub(crate) fn define() -> TargetIsa {
    let settings = SettingGroupBuilder::new("spirv");
    TargetIsa::new("spirv", settings.build())
}
