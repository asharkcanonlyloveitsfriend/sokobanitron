#[cfg(feature = "stage-profile")]
mod enabled {
    use std::collections::BTreeMap;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    type StatsMap = BTreeMap<&'static str, (u64, u128)>;

    fn stats() -> &'static Mutex<StatsMap> {
        static STATS: OnceLock<Mutex<StatsMap>> = OnceLock::new();
        STATS.get_or_init(|| Mutex::new(BTreeMap::new()))
    }

    pub fn record(stage: &'static str, dur: Duration) {
        let nanos = dur.as_nanos();
        let mut guard = stats().lock().unwrap();
        let entry = guard.entry(stage).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += nanos;
    }

    fn reset_stats() {
        stats().lock().unwrap().clear();
    }

    fn render_stats() -> String {
        let guard = stats().lock().unwrap();
        if guard.is_empty() {
            return "no samples collected".to_string();
        }

        let total_ns: u128 = guard.values().map(|(_, ns)| *ns).sum();
        let mut out = String::new();
        out.push_str("Stage profile (aggregate)\n");
        out.push_str("stage\tcalls\ttotal_ms\tavg_us\tpct\n");
        for (stage, (calls, ns)) in guard.iter() {
            let total_ms = *ns as f64 / 1_000_000.0;
            let avg_us = (*ns as f64 / *calls as f64) / 1_000.0;
            let pct = if total_ns == 0 {
                0.0
            } else {
                (*ns as f64 * 100.0) / total_ns as f64
            };
            out.push_str(&format!(
                "{stage}\t{calls}\t{total_ms:.3}\t{avg_us:.3}\t{pct:.1}\n"
            ));
        }
        out.push_str(&format!(
            "TOTAL\t-\t{:.3}\t-\t100.0\n",
            total_ns as f64 / 1_000_000.0
        ));
        out
    }

    pub fn reset() {
        reset_stats();
    }

    pub fn report() -> String {
        render_stats()
    }
}

#[cfg(feature = "stage-profile")]
pub use enabled::{record, report, reset};

// When the feature is disabled, provide zero-cost no-op stubs.
#[cfg(not(feature = "stage-profile"))]
#[allow(dead_code)]
pub fn record(_stage: &'static str, _dur: std::time::Duration) {}

#[cfg(not(feature = "stage-profile"))]
#[allow(dead_code)]
pub fn reset() {}

#[cfg(not(feature = "stage-profile"))]
#[allow(dead_code)]
pub fn report() -> String {
    "stage profiling feature disabled".to_string()
}
