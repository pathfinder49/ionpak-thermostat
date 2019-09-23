use crate::board::pwm::PwmChannel;

/// Thermo-Electric Cooling device controlled through PWM
pub struct TEC<MaxIPos: PwmChannel, MaxINeg: PwmChannel, ISet: PwmChannel, MaxV: PwmChannel> {
    pub max_i_pos: MaxIPos,
    pub max_i_neg: MaxINeg,
    pub i_set: ISet,
    pub max_v: MaxV,
}
