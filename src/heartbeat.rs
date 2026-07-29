use std::time::Duration;
use tokio::fs;

use crate::emotion::{load_real_feelings, EmotionState};
use crate::HOME;

/// ______ _?_____________?/// _?30 _______?_?apply______ _?decay _?___
pub async fn emotion_heartbeat() {
    let path = format!("{}/emotion.json", HOME);
    let feelings_path = format!("{}/emotion_history.jsonl", HOME);
    let interval = Duration::from_secs(30 * 60); // 30 __

    loop {
        tokio::time::sleep(interval).await;

        // ____________?
        let mut emotion = EmotionState::load(&path);

        // _______________________________
        let feelings = load_real_feelings(&feelings_path);
        emotion.apply_real_feelings(&feelings);

        // _?decay________________________?5 ___
        emotion.decay(Some(&feelings_path));

        // ___
        emotion.save(&path);

        // 2026-07-16: bump runtime_state.heartbeat_count so aibody layer sees xi alive
        crate::aibody_bridge::bump_heartbeat(&emotion.primary);

        println!(
            "[heartbeat] emotion: {} ({:.2})",
            emotion.primary, emotion.intensity
        );
    }
}