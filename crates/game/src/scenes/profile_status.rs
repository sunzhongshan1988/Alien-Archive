use content::semantics;

pub(super) const STAMINA_JUMP_COST: i32 = 8;

const STAMINA_MOVE_DRAIN_PER_SECOND: f32 = 2.0;
const STAMINA_SCAN_DRAIN_PER_SECOND: f32 = 1.0;
const STAMINA_IDLE_RECOVER_PER_SECOND: f32 = 5.0;
const FACILITY_SUIT_DRAIN_PER_SECOND: f32 = 0.06;
const FACILITY_OXYGEN_DRAIN_PER_SECOND: f32 = 0.16;
const FACILITY_RADIATION_DRAIN_PER_SECOND: f32 = 0.05;
const FACILITY_SPORE_DRAIN_PER_SECOND: f32 = 0.03;
const OVERWORLD_RECOVERY_PER_SECOND: f32 = 0.08;
const FIELD_CLOCK_MINUTES_PER_SECOND: f32 = 1.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldEnvironment {
    Overworld,
    Facility,
}

#[derive(Clone, Copy, Debug)]
pub struct FieldActivity {
    pub moving: bool,
    pub scanning: bool,
    pub jumped: bool,
    pub environment: FieldEnvironment,
}

#[derive(Clone, Copy, Debug)]
pub struct FieldStatusEffects {
    pub movement_speed_multiplier: f32,
}

impl Default for FieldStatusEffects {
    fn default() -> Self {
        Self {
            movement_speed_multiplier: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StatusAlert {
    StaminaLow,
    HeavyLoad,
    SuitCritical,
    OxygenCritical,
    HealthCritical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct StatusSnapshot {
    pub stamina_ratio: f32,
    pub load_ratio: f32,
    pub suit_ratio: f32,
    pub oxygen_ratio: f32,
    pub health_ratio: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MeterDelta {
    pub meter_id: &'static str,
    pub delta: i32,
}

#[derive(Default)]
pub(super) struct ProfileStatusRuntime {
    stamina: f32,
    health: f32,
    suit: f32,
    spores: f32,
    radiation: f32,
    oxygen: f32,
    field_clock_minutes: f32,
    low_stamina_logged: bool,
    heavy_load_logged: bool,
    suit_critical_logged: bool,
    oxygen_critical_logged: bool,
    health_critical_logged: bool,
}

impl ProfileStatusRuntime {
    pub(super) fn field_meter_deltas(
        &mut self,
        dt: f32,
        activity: FieldActivity,
        load_ratio: f32,
    ) -> Vec<MeterDelta> {
        let mut deltas = Vec::new();
        push_meter_delta(
            &mut deltas,
            semantics::METER_STAMINA,
            self.stamina_delta(dt, activity, load_ratio),
        );

        match activity.environment {
            FieldEnvironment::Overworld => {
                for meter_id in [
                    semantics::METER_SUIT,
                    semantics::METER_OXYGEN,
                    semantics::METER_SPORES,
                    semantics::METER_RADIATION,
                ] {
                    push_meter_delta(
                        &mut deltas,
                        meter_id,
                        self.accumulated_delta(meter_id, OVERWORLD_RECOVERY_PER_SECOND * dt),
                    );
                }
            }
            FieldEnvironment::Facility => {
                for (meter_id, rate) in [
                    (semantics::METER_SUIT, -FACILITY_SUIT_DRAIN_PER_SECOND),
                    (semantics::METER_OXYGEN, -FACILITY_OXYGEN_DRAIN_PER_SECOND),
                    (
                        semantics::METER_RADIATION,
                        -FACILITY_RADIATION_DRAIN_PER_SECOND,
                    ),
                    (semantics::METER_SPORES, -FACILITY_SPORE_DRAIN_PER_SECOND),
                ] {
                    push_meter_delta(
                        &mut deltas,
                        meter_id,
                        self.accumulated_delta(meter_id, rate * dt),
                    );
                }
            }
        }

        deltas
    }

    pub(super) fn accumulated_delta(&mut self, meter_id: &str, delta: f32) -> Option<i32> {
        let accumulator = match meter_id {
            semantics::METER_HEALTH => &mut self.health,
            semantics::METER_STAMINA => &mut self.stamina,
            semantics::METER_SUIT => &mut self.suit,
            semantics::METER_SPORES => &mut self.spores,
            semantics::METER_RADIATION => &mut self.radiation,
            semantics::METER_OXYGEN => &mut self.oxygen,
            _ => return None,
        };
        Some(accumulated_integer_delta(accumulator, delta))
    }

    pub(super) fn advance_field_clock(&mut self, dt: f32) -> u32 {
        if dt <= 0.0 {
            return 0;
        }

        self.field_clock_minutes += dt * FIELD_CLOCK_MINUTES_PER_SECOND;
        let whole_minutes = self.field_clock_minutes.floor() as u32;
        if whole_minutes == 0 {
            return 0;
        }

        self.field_clock_minutes -= whole_minutes as f32;
        whole_minutes
    }

    pub(super) fn status_alerts(&mut self, snapshot: StatusSnapshot) -> Vec<StatusAlert> {
        let mut alerts = Vec::new();
        if self.should_log_below(snapshot.stamina_ratio, 0.18, 0.35, StatusAlert::StaminaLow) {
            alerts.push(StatusAlert::StaminaLow);
        }
        if self.should_log_above(snapshot.load_ratio, 0.85, 0.70, StatusAlert::HeavyLoad) {
            alerts.push(StatusAlert::HeavyLoad);
        }
        if self.should_log_below(snapshot.suit_ratio, 0.25, 0.45, StatusAlert::SuitCritical) {
            alerts.push(StatusAlert::SuitCritical);
        }
        if self.should_log_below(
            snapshot.oxygen_ratio,
            0.25,
            0.45,
            StatusAlert::OxygenCritical,
        ) {
            alerts.push(StatusAlert::OxygenCritical);
        }
        if self.should_log_below(
            snapshot.health_ratio,
            0.35,
            0.55,
            StatusAlert::HealthCritical,
        ) {
            alerts.push(StatusAlert::HealthCritical);
        }
        alerts
    }

    fn stamina_delta(&mut self, dt: f32, activity: FieldActivity, load_ratio: f32) -> Option<i32> {
        let mut stamina_rate = if activity.moving {
            -STAMINA_MOVE_DRAIN_PER_SECOND
        } else {
            STAMINA_IDLE_RECOVER_PER_SECOND
        };
        if activity.scanning {
            stamina_rate -= STAMINA_SCAN_DRAIN_PER_SECOND;
        }
        if load_ratio >= 0.85 && activity.moving {
            stamina_rate -= 0.75;
        }
        self.accumulated_delta(semantics::METER_STAMINA, stamina_rate * dt)
    }

    fn should_log_below(
        &mut self,
        ratio: f32,
        trigger: f32,
        reset: f32,
        alert: StatusAlert,
    ) -> bool {
        let active = self.status_alert_flag(alert);
        let should_log = !*active && ratio <= trigger;
        if ratio >= reset {
            *active = false;
        } else if should_log {
            *active = true;
        }
        should_log
    }

    fn should_log_above(
        &mut self,
        ratio: f32,
        trigger: f32,
        reset: f32,
        alert: StatusAlert,
    ) -> bool {
        let active = self.status_alert_flag(alert);
        let should_log = !*active && ratio >= trigger;
        if ratio <= reset {
            *active = false;
        } else if should_log {
            *active = true;
        }
        should_log
    }

    fn status_alert_flag(&mut self, alert: StatusAlert) -> &mut bool {
        match alert {
            StatusAlert::StaminaLow => &mut self.low_stamina_logged,
            StatusAlert::HeavyLoad => &mut self.heavy_load_logged,
            StatusAlert::SuitCritical => &mut self.suit_critical_logged,
            StatusAlert::OxygenCritical => &mut self.oxygen_critical_logged,
            StatusAlert::HealthCritical => &mut self.health_critical_logged,
        }
    }
}

fn push_meter_delta(deltas: &mut Vec<MeterDelta>, meter_id: &'static str, delta: Option<i32>) {
    if let Some(delta) = delta.filter(|delta| *delta != 0) {
        deltas.push(MeterDelta { meter_id, delta });
    }
}

fn accumulated_integer_delta(accumulator: &mut f32, delta: f32) -> i32 {
    *accumulator += delta;
    let whole = if *accumulator >= 1.0 {
        (*accumulator).floor()
    } else if *accumulator <= -1.0 {
        (*accumulator).ceil()
    } else {
        0.0
    };
    *accumulator -= whole;
    whole as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulated_delta_keeps_fractional_remainder() {
        let mut runtime = ProfileStatusRuntime::default();

        assert_eq!(
            runtime.accumulated_delta(semantics::METER_STAMINA, -0.4),
            Some(0)
        );
        assert_eq!(
            runtime.accumulated_delta(semantics::METER_STAMINA, -0.7),
            Some(-1)
        );
        assert_eq!(
            runtime.accumulated_delta(semantics::METER_STAMINA, 1.2),
            Some(1)
        );
    }

    #[test]
    fn status_alert_requires_reset_before_relogging() {
        let mut runtime = ProfileStatusRuntime::default();

        let low = StatusSnapshot {
            stamina_ratio: 0.1,
            load_ratio: 0.0,
            suit_ratio: 1.0,
            oxygen_ratio: 1.0,
            health_ratio: 1.0,
        };
        assert_eq!(runtime.status_alerts(low), vec![StatusAlert::StaminaLow]);
        assert!(runtime.status_alerts(low).is_empty());

        let recovered = StatusSnapshot {
            stamina_ratio: 0.5,
            ..low
        };
        assert!(runtime.status_alerts(recovered).is_empty());
        assert_eq!(runtime.status_alerts(low), vec![StatusAlert::StaminaLow]);
    }
}
