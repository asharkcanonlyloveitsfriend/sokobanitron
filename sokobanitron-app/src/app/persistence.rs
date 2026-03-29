use super::{AppState, AppliedUpdate};
use crate::gameplay::set_gameplay_level_sets;
use crate::level_bootstrap::build_preview_boards;
use crate::persistence::LevelPersistence;
use sokobanitron_gameplay::{BoardView, GameplayController};
use std::io;

struct ActivatedLevelSet {
    pub controller: GameplayController,
    pub preview_boards: Vec<BoardView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RuntimeEffects {
    pub needs_gameplay_render: bool,
}

fn persist_runtime_changes(
    level_persistence: &mut LevelPersistence,
    applied: &AppliedUpdate,
    controller: &GameplayController,
) -> io::Result<()> {
    let mut errors = Vec::new();

    if let Some(index) = applied.persistence.resume_level_changed
        && let Err(err) = level_persistence.persist_resume_level(index)
    {
        errors.push(format!("persist resume level failed: {err}"));
    }

    if let Some(level_index) = applied.persistence.solved_level {
        let solution_history = controller.solution_history();
        if let Err(err) = level_persistence.record_completion(level_index, &solution_history) {
            errors.push(format!("record completion failed: {err}"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(errors.join("; ")))
    }
}

fn sync_level_set_catalog(app_state: &mut AppState, level_persistence: &LevelPersistence) {
    set_gameplay_level_sets(
        &mut app_state.gameplay,
        level_persistence.level_set_catalog(),
        level_persistence.active_level_set_index().unwrap_or(0),
    );
}

fn activate_level_set_for_app(
    level_persistence: &mut LevelPersistence,
    selected_index: usize,
) -> io::Result<Option<ActivatedLevelSet>> {
    let Some(loaded) = level_persistence.switch_to_level_set(selected_index)? else {
        return Ok(None);
    };

    let preview_boards = build_preview_boards(&loaded.levels);
    let controller = GameplayController::new_at_level(
        loaded.levels,
        loaded.initial_level_index,
        loaded.persisted_resume_level_index,
    );
    Ok(Some(ActivatedLevelSet {
        controller,
        preview_boards,
    }))
}

pub(crate) fn apply_runtime_effects(
    controller: &mut GameplayController,
    app_state: &mut AppState,
    level_persistence: &mut LevelPersistence,
    preview_boards: &mut Vec<BoardView>,
    applied: &AppliedUpdate,
) -> io::Result<RuntimeEffects> {
    let mut effects = RuntimeEffects::default();
    let mut errors = Vec::new();

    if let Some(selected_index) = applied.level_set_selected {
        match activate_level_set_for_app(level_persistence, selected_index) {
            Ok(Some(activated)) => {
                *controller = activated.controller;
                *preview_boards = activated.preview_boards;
                effects.needs_gameplay_render = true;
            }
            Ok(None) => {}
            Err(err) => errors.push(format!("level-set activation failed: {err}")),
        }
    }

    if let Err(err) = persist_runtime_changes(level_persistence, applied, controller) {
        errors.push(format!("persistence write failed: {err}"));
    }

    // Catalog sync is an in-memory mirror of the persistence state, so keep it as the final step
    // after any level-set activation and persistence writes have settled.
    sync_level_set_catalog(app_state, level_persistence);

    if errors.is_empty() {
        Ok(effects)
    } else {
        Err(io::Error::other(errors.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::apply_runtime_effects;
    use crate::app::{AppState, AppliedUpdate};
    use crate::gameplay::set_gameplay_level_sets;
    use crate::level_bootstrap::load_initial_levels_for_app;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn browsing_level_set_does_not_persist_active_set_selection() {
        let root = temp_dir("browse-level-set");
        let inbox = root.join("to_import");
        fs::create_dir_all(&inbox).expect("create inbox");
        fs::write(
            inbox.join("alpha.slc"),
            r#"
                <SokobanLevels>
                  <Title>Alpha</Title>
                  <LevelCollection>
                    <Level Id="1">
                      <L>#####</L>
                      <L>#@$.#</L>
                      <L>#####</L>
                    </Level>
                  </LevelCollection>
                </SokobanLevels>
            "#,
        )
        .expect("write alpha");
        fs::write(
            inbox.join("beta.slc"),
            r#"
                <SokobanLevels>
                  <Title>Beta</Title>
                  <LevelCollection>
                    <Level Id="1">
                      <L>#######</L>
                      <L>#@  $.#</L>
                      <L>#######</L>
                    </Level>
                    <Level Id="2">
                      <L>#######</L>
                      <L>#@ $. #</L>
                      <L>#######</L>
                    </Level>
                  </LevelCollection>
                </SokobanLevels>
            "#,
        )
        .expect("write beta");

        let initial = load_initial_levels_for_app(&root);
        let mut controller = sokobanitron_gameplay::GameplayController::new_at_level(
            initial.levels.clone(),
            initial.initial_level_index,
            initial.persisted_resume_level_index,
        );
        let mut app_state = AppState::default();
        set_gameplay_level_sets(
            &mut app_state.gameplay,
            initial.level_set_catalog.clone(),
            initial.active_level_set_index,
        );
        let mut level_persistence = initial.persistence;
        let mut preview_boards = initial.preview_boards;

        let applied = AppliedUpdate {
            changes: Default::default(),
            persistence: Default::default(),
            level_set_selected: Some(1),
            presentation_plan: None,
            rendered_frame: false,
        };

        let effects = apply_runtime_effects(
            &mut controller,
            &mut app_state,
            &mut level_persistence,
            &mut preview_boards,
            &applied,
        )
        .expect("apply runtime update");

        assert!(effects.needs_gameplay_render);
        assert_eq!(app_state.gameplay.active_level_set, 1);
        assert_eq!(controller.level_count(), 2);
        assert_eq!(preview_boards.len(), 2);

        drop(level_persistence);

        let reloaded = load_initial_levels_for_app(&root);
        assert_eq!(reloaded.active_level_set_index, 0);
        assert_eq!(reloaded.initial_level_index, 0);

        fs::remove_dir_all(root).expect("cleanup");
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("sokobanitron-app-{name}-{nanos}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
