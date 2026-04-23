use super::action::AppAction;
use super::input::{AppInput, interpret_input};
use super::persistence::{
    apply_runtime_effects, persist_editor_puzzle_to_default_set, refresh_level_set_for_app,
    sync_level_set_catalog_for_app,
};
use super::presentation::{
    AppFrameRenderer, FrameDamage, GameplayAnimationPolicy, PresentationPlan, RendererOverrides,
};
use super::presentation::{FrameRequest, FrameSink, render_presentation_plan};
use super::reducer::PersistenceUpdate;
use super::reducer::apply_action;
use super::state::{AppInteractionMode, AppScreen, AppState};
use crate::editor::{
    EditorUiAction, build_current_editor_frame_request, editor_cursor_moved, editor_mouse_pressed,
    editor_mouse_released, editor_touch, reset_editor_interaction_state, resize_editor_surface,
    set_editor_double_tap_window, set_editor_touch_slop,
};
use crate::gameplay::{
    build_current_gameplay_board_frame_request, build_current_gameplay_screen_frame_request,
    build_sleep_gameplay_frame_request, interpret_gameplay_pointer_event,
    interpret_gameplay_pointer_tap, resize_gameplay_surface, set_gameplay_level_sets,
    set_gameplay_max_cell_size, set_gameplay_touch_slop,
};
use crate::level_bootstrap::InitialLevels;
use crate::persistence::LevelPersistence;
use crate::shared::PointerPhase;
use sokobanitron_gameplay::{BoardView, GameplayController, GameplayControllerChanges};
use sokobanitron_level_editor::{EditorCommand, LevelEditor};
use std::time::Duration;

const EDITOR_HINT_ADVANCE_STEPS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderWorkResult {
    /// `true` means the continuation already rendered the next visible frame, so callers should
    /// not do a fallback current-frame render for this tick.
    pub frame_changed: bool,
    pub needs_followup_wake: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedUpdate {
    pub changes: GameplayControllerChanges,
    pub persistence: PersistenceUpdate,
    pub level_set_selected: Option<usize>,
    pub presentation_plan: Option<PresentationPlan>,
    pub rendered_frame: bool,
    pub render_work: RenderWorkResult,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SharedAppRendererConfig {
    pub renderer_overrides: RendererOverrides,
    pub gameplay_animation_policy: GameplayAnimationPolicy,
}

impl SharedAppRendererConfig {
    fn build_renderer(self) -> AppFrameRenderer {
        AppFrameRenderer::with_renderer_overrides_and_gameplay_animation_policy(
            self.renderer_overrides,
            self.gameplay_animation_policy,
        )
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SharedAppRuntimeConfig {
    pub editor_available: bool,
    pub supports_multi_touch: bool,
    pub gameplay_touch_slop_px: Option<i32>,
    pub gameplay_max_cell_size: Option<u32>,
    pub editor_touch_slop_px: Option<i32>,
    pub editor_double_tap_window: Option<Duration>,
    pub renderer: SharedAppRendererConfig,
}

impl SharedAppRuntimeConfig {
    fn build_app_state(self) -> AppState {
        let mut app_state = AppState {
            editor_available: self.editor_available,
            supports_multi_touch: self.supports_multi_touch,
            ..AppState::default()
        };
        if let Some(max_cell_size) = self.gameplay_max_cell_size {
            set_gameplay_max_cell_size(&mut app_state.gameplay, max_cell_size);
        }
        if let Some(touch_slop) = self.gameplay_touch_slop_px {
            set_gameplay_touch_slop(&mut app_state.gameplay, touch_slop);
        }
        if let Some(touch_slop) = self.editor_touch_slop_px {
            set_editor_touch_slop(&mut app_state, touch_slop);
        }
        if let Some(double_tap_window) = self.editor_double_tap_window {
            set_editor_double_tap_window(&mut app_state, double_tap_window);
        }
        app_state
    }
}

trait AppDriverContext {
    type Error;

    fn app_runtime_mut(&mut self) -> AppRuntimeMut<'_>;

    fn editor_runtime_mut(&mut self) -> Option<EditorAppRuntimeMut<'_>> {
        None
    }

    fn warn(&mut self, message: &str) {
        eprintln!("warning: {message}");
    }

    fn has_pending_frame_presentation(&mut self) -> bool {
        false
    }

    fn continue_frame_presentation_and_render(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AppPointerInput {
    CursorMoved {
        x: f64,
        y: f64,
    },
    MousePressed {
        x: f64,
        y: f64,
    },
    MouseReleased,
    Pointer {
        id: u64,
        phase: PointerPhase,
        x: f64,
        y: f64,
    },
}

struct AppRuntimeMut<'a> {
    pub controller: &'a mut GameplayController,
    pub app_state: &'a mut AppState,
    pub level_persistence: &'a mut LevelPersistence,
    pub preview_boards: &'a mut Vec<BoardView>,
}

struct EditorAppRuntimeMut<'a> {
    pub app: AppRuntimeMut<'a>,
    pub editor: &'a mut LevelEditor,
}

/// Platform-owned frame output for app-rendered frames.
///
/// The shared runtime draws into its gray frame, then calls this boundary so clients can copy,
/// accumulate damage, or present to a platform surface.
pub trait AppFramePresenter {
    type Error;

    fn present_frame(
        &mut self,
        damage: FrameDamage,
        gray_frame: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error>;
}

/// Shared app runtime state that clients can embed next to platform-specific handles.
///
/// It owns the app model, editor, persistence-facing state, preview boards, gray frame, surface
/// size, and frame renderer. Clients still decide when to wake and how to present the rendered
/// frame to their platform surface.
pub struct SharedAppRuntime {
    controller: GameplayController,
    app_state: AppState,
    level_persistence: LevelPersistence,
    preview_boards: Vec<BoardView>,
    editor: LevelEditor,
    frame_renderer: AppFrameRenderer,
    gray_frame: Vec<u8>,
    surface_width: u32,
    surface_height: u32,
}

impl<'a> AppRuntimeMut<'a> {
    fn with_editor(self, editor: &'a mut LevelEditor) -> EditorAppRuntimeMut<'a> {
        EditorAppRuntimeMut { app: self, editor }
    }
}

impl SharedAppRuntime {
    pub fn new(
        initial_levels: InitialLevels,
        surface_width: u32,
        surface_height: u32,
        config: SharedAppRuntimeConfig,
    ) -> Self {
        let mut app_state = config.build_app_state();
        let frame_renderer = config.renderer.build_renderer();
        let surface_width = surface_width.max(1);
        let surface_height = surface_height.max(1);
        resize_gameplay_surface(&mut app_state.gameplay, surface_width, surface_height);
        resize_editor_surface(&mut app_state, surface_width, surface_height);
        set_gameplay_level_sets(
            &mut app_state.gameplay,
            initial_levels.level_set_catalog,
            Some(initial_levels.active_level_set_index),
        );
        let controller = GameplayController::new_at_level(
            initial_levels.levels,
            initial_levels.initial_level_index,
            initial_levels.persisted_resume_level_index,
        );

        Self {
            controller,
            app_state,
            level_persistence: initial_levels.persistence,
            preview_boards: initial_levels.preview_boards,
            editor: LevelEditor::new(),
            frame_renderer,
            gray_frame: allocate_gray_frame(surface_width, surface_height),
            surface_width,
            surface_height,
        }
    }

    pub fn surface_width(&self) -> u32 {
        self.surface_width
    }

    pub fn surface_height(&self) -> u32 {
        self.surface_height
    }

    pub fn gray_frame(&self) -> &[u8] {
        &self.gray_frame
    }

    pub fn resize_surface(&mut self, surface_width: u32, surface_height: u32) {
        self.surface_width = surface_width.max(1);
        self.surface_height = surface_height.max(1);
        self.gray_frame = allocate_gray_frame(self.surface_width, self.surface_height);
        resize_gameplay_surface(
            &mut self.app_state.gameplay,
            self.surface_width,
            self.surface_height,
        );
        resize_editor_surface(&mut self.app_state, self.surface_width, self.surface_height);
        self.frame_renderer.clear_gameplay_presentation_state();
    }

    fn app_runtime_mut(&mut self) -> AppRuntimeMut<'_> {
        AppRuntimeMut {
            controller: &mut self.controller,
            app_state: &mut self.app_state,
            level_persistence: &mut self.level_persistence,
            preview_boards: &mut self.preview_boards,
        }
    }

    fn editor_runtime_mut(&mut self) -> EditorAppRuntimeMut<'_> {
        AppRuntimeMut {
            controller: &mut self.controller,
            app_state: &mut self.app_state,
            level_persistence: &mut self.level_persistence,
            preview_boards: &mut self.preview_boards,
        }
        .with_editor(&mut self.editor)
    }

    fn current_frame_request(&self) -> FrameRequest {
        build_current_app_screen_frame_request(&self.controller, &self.app_state, &self.editor)
    }

    fn sleep_gameplay_frame_request(&self) -> FrameRequest {
        build_sleep_gameplay_frame_request(&self.controller, &self.app_state)
    }

    fn draw_frame_request(&mut self, request: &FrameRequest) -> FrameDamage {
        self.frame_renderer.draw_frame_request(
            &mut self.gray_frame,
            self.surface_width,
            self.surface_height,
            request,
            &self.preview_boards,
        )
    }

    fn draw_full_frame_request(&mut self, request: &FrameRequest) -> FrameDamage {
        self.frame_renderer.draw_full_frame_request(
            &mut self.gray_frame,
            self.surface_width,
            self.surface_height,
            request,
            &self.preview_boards,
        )
    }

    fn has_pending_visible_presentation(&self) -> bool {
        self.frame_renderer
            .has_pending_visible_presentation(&self.app_state)
    }

    fn draw_pending_visible_presentation(&mut self) -> FrameDamage {
        self.frame_renderer.draw_pending_visible_presentation(
            &self.app_state,
            &mut self.gray_frame,
            self.surface_width,
            self.surface_height,
        )
    }

    fn render_frame_request<P: AppFramePresenter>(
        &mut self,
        request: &FrameRequest,
        presenter: &mut P,
    ) -> Result<(), P::Error> {
        let damage = self.draw_frame_request(request);
        self.output_drawn_frame(damage, presenter)
    }

    fn render_full_frame_request<P: AppFramePresenter>(
        &mut self,
        request: &FrameRequest,
        presenter: &mut P,
    ) -> Result<(), P::Error> {
        self.draw_full_frame_request(request);
        self.output_drawn_frame(FrameDamage::Full, presenter)
    }

    pub fn render_current_frame<P: AppFramePresenter>(
        &mut self,
        presenter: &mut P,
    ) -> Result<(), P::Error> {
        let request = self.current_frame_request();
        self.render_frame_request(&request, presenter)
    }

    pub fn render_full_current_frame<P: AppFramePresenter>(
        &mut self,
        presenter: &mut P,
    ) -> Result<(), P::Error> {
        let request = self.current_frame_request();
        self.render_full_frame_request(&request, presenter)
    }

    pub fn render_sleep_gameplay_frame<P: AppFramePresenter>(
        &mut self,
        presenter: &mut P,
    ) -> Result<(), P::Error> {
        let request = self.sleep_gameplay_frame_request();
        self.render_frame_request(&request, presenter)
    }

    fn render_pending_visible_presentation<P: AppFramePresenter>(
        &mut self,
        presenter: &mut P,
    ) -> Result<FrameDamage, P::Error> {
        let damage = self.draw_pending_visible_presentation();
        self.output_drawn_frame(damage, presenter)?;
        Ok(damage)
    }

    pub fn apply_input_and_render<P: AppFramePresenter>(
        &mut self,
        input: AppInput,
        presenter: &mut P,
    ) -> Result<AppliedUpdate, P::Error> {
        let mut context = SharedAppRuntimeContext {
            runtime: self,
            presenter,
        };
        apply_input_and_render_in_context(&mut context, input)
    }

    pub fn handle_pointer_input_and_render<P: AppFramePresenter>(
        &mut self,
        input: AppPointerInput,
        presenter: &mut P,
    ) -> Result<RenderWorkResult, P::Error> {
        let mut context = SharedAppRuntimeContext {
            runtime: self,
            presenter,
        };
        handle_pointer_input_and_render_in_context(&mut context, input)
    }

    pub fn continue_pending_render_work_and_render<P: AppFramePresenter>(
        &mut self,
        presenter: &mut P,
    ) -> Result<RenderWorkResult, P::Error> {
        let mut context = SharedAppRuntimeContext {
            runtime: self,
            presenter,
        };
        continue_pending_render_work_and_render_in_context(&mut context)
    }

    pub fn has_pending_render_work(&self) -> bool {
        self.has_pending_visible_presentation() || self.has_pending_editor_hint_job()
    }

    fn has_pending_editor_hint_job(&self) -> bool {
        self.app_state.is_editor_screen() && self.editor.has_active_pull_hint_job()
    }

    fn output_drawn_frame<P: AppFramePresenter>(
        &self,
        damage: FrameDamage,
        presenter: &mut P,
    ) -> Result<(), P::Error> {
        presenter.present_frame(
            damage,
            &self.gray_frame,
            self.surface_width,
            self.surface_height,
        )
    }
}

struct SharedAppRuntimeContext<'a, 'p, P> {
    runtime: &'a mut SharedAppRuntime,
    presenter: &'p mut P,
}

impl<P: AppFramePresenter> AppDriverContext for SharedAppRuntimeContext<'_, '_, P> {
    type Error = P::Error;

    fn app_runtime_mut(&mut self) -> AppRuntimeMut<'_> {
        self.runtime.app_runtime_mut()
    }

    fn editor_runtime_mut(&mut self) -> Option<EditorAppRuntimeMut<'_>> {
        Some(self.runtime.editor_runtime_mut())
    }

    fn has_pending_frame_presentation(&mut self) -> bool {
        self.runtime.has_pending_visible_presentation()
    }

    fn continue_frame_presentation_and_render(&mut self) -> Result<bool, Self::Error> {
        if !self.runtime.has_pending_visible_presentation() {
            return Ok(false);
        }
        self.runtime
            .render_pending_visible_presentation(self.presenter)?;
        Ok(true)
    }
}

impl<P: AppFramePresenter> FrameSink for SharedAppRuntimeContext<'_, '_, P> {
    type Error = P::Error;

    fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error> {
        self.runtime.render_frame_request(request, self.presenter)
    }
}

fn allocate_gray_frame(surface_width: u32, surface_height: u32) -> Vec<u8> {
    vec![0; frame_len(surface_width, surface_height)]
}

fn frame_len(surface_width: u32, surface_height: u32) -> usize {
    usize::try_from(surface_width)
        .expect("surface width should fit usize")
        .saturating_mul(usize::try_from(surface_height).expect("surface height should fit usize"))
}

fn apply_action_in_context<C: AppDriverContext>(
    context: &mut C,
    action: AppAction,
) -> Result<AppliedUpdate, C::Error> {
    let update = {
        let runtime = context.app_runtime_mut();
        apply_action(runtime.controller, runtime.app_state, action)
    };

    Ok(AppliedUpdate {
        changes: update.changes,
        persistence: update.persistence,
        level_set_selected: update.level_set_selected,
        presentation_plan: update.presentation_plan,
        rendered_frame: false,
        render_work: RenderWorkResult::default(),
    })
}

fn build_current_app_screen_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
    editor: &LevelEditor,
) -> FrameRequest {
    match app_state.active_screen() {
        AppScreen::Gameplay => build_current_gameplay_screen_frame_request(controller, app_state),
        AppScreen::Editor => build_current_editor_frame_request(app_state, editor),
    }
}

fn apply_action_and_render_in_context<C>(
    context: &mut C,
    action: AppAction,
) -> Result<AppliedUpdate, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    let before_request = build_current_context_screen_frame_request(context);
    let mut applied = apply_action_in_context(context, action)?;

    let runtime_effects = {
        let runtime = context.app_runtime_mut();
        apply_runtime_effects(
            runtime.controller,
            runtime.app_state,
            runtime.level_persistence,
            runtime.preview_boards,
            &applied,
        )
    };
    let needs_gameplay_render = match runtime_effects {
        Ok(effects) => effects.needs_gameplay_render,
        Err(err) => {
            context.warn(&format!(
                "failed to apply post-action runtime effects: {err}"
            ));
            false
        }
    };

    if let Some(plan) = applied.presentation_plan.as_ref() {
        render_presentation_plan(context, plan)?;
        applied.rendered_frame = true;
    } else if needs_gameplay_render {
        // Runtime effects may replace controller state for in-session browsing, so they must run
        // before we build this fallback gameplay frame.
        let request = {
            let runtime = context.app_runtime_mut();
            build_current_gameplay_board_frame_request(runtime.controller, runtime.app_state)
        };
        context.render_frame(&request)?;
        applied.rendered_frame = true;
    } else {
        applied.rendered_frame = render_current_screen_frame_if_changed(context, before_request)?;
    }
    applied.render_work = current_render_work_result(context, applied.rendered_frame);
    Ok(applied)
}

fn apply_editor_ui_action(action: Option<EditorUiAction>, runtime: EditorAppRuntimeMut<'_>) {
    if let Some(EditorUiAction::SavePuzzle) = action {
        let saved =
            persist_editor_puzzle_to_default_set(runtime.app.level_persistence, runtime.editor)
                .expect("editor save button should only be available for an exportable puzzle");
        let saved_set_was_active =
            runtime.app.app_state.gameplay.active_level_set == Some(saved.level_set_index);
        sync_level_set_catalog_for_app(runtime.app.app_state, runtime.app.level_persistence);
        if saved_set_was_active {
            refresh_level_set_for_app(
                runtime.app.controller,
                runtime.app.level_persistence,
                runtime.app.preview_boards,
                saved.level_set_index,
            )
            .expect("saved active level set should reload immediately");
        }
        *runtime.editor = LevelEditor::new();
        reset_editor_interaction_state(runtime.app.app_state);
    }
}

fn apply_input_and_render_in_context<C>(
    context: &mut C,
    input: AppInput,
) -> Result<AppliedUpdate, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    let action = {
        let runtime = context.app_runtime_mut();
        interpret_input(runtime.app_state, input)
    };
    apply_action_and_render_in_context(context, action)
}

fn continue_pending_render_work_and_render_in_context<C>(
    context: &mut C,
) -> Result<RenderWorkResult, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    if context.has_pending_frame_presentation() {
        let frame_changed = context.continue_frame_presentation_and_render()?;
        return Ok(current_render_work_result(context, frame_changed));
    }

    let before_request = build_current_context_screen_frame_request(context);
    let advanced = {
        let Some(runtime) = context.editor_runtime_mut() else {
            return Ok(RenderWorkResult::default());
        };
        if !runtime.app.app_state.is_editor_screen() || !runtime.editor.has_active_pull_hint_job() {
            false
        } else {
            runtime.editor.apply_command(EditorCommand::AdvanceHintJob {
                steps: EDITOR_HINT_ADVANCE_STEPS,
            });
            true
        }
    };
    if !advanced {
        return Ok(RenderWorkResult::default());
    }
    let frame_changed = render_current_screen_frame_if_changed(context, before_request)?;
    Ok(current_render_work_result(context, frame_changed))
}

fn has_pending_render_work_in_context<C: AppDriverContext>(context: &mut C) -> bool {
    context.has_pending_frame_presentation() || has_pending_editor_hint_job_in_context(context)
}

fn handle_pointer_input_and_render_in_context<C>(
    context: &mut C,
    input: AppPointerInput,
) -> Result<RenderWorkResult, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    match input {
        AppPointerInput::CursorMoved { x, y } => {
            if matches!(
                current_interaction_mode(context),
                AppInteractionMode::Editor
            ) {
                return mutate_editor_and_render_in_context(context, |app_state, editor| {
                    editor_cursor_moved(app_state, editor, x, y);
                    None
                });
            }
        }
        AppPointerInput::MousePressed { x, y } => match current_interaction_mode(context) {
            AppInteractionMode::Gameplay => {
                return handle_gameplay_mouse_pressed_in_context(context, x, y);
            }
            AppInteractionMode::Editor => {
                return mutate_editor_and_render_in_context(context, |app_state, editor| {
                    editor_mouse_pressed(app_state, editor, x, y)
                });
            }
            AppInteractionMode::Overlay(overlay) => match overlay.owning_screen() {
                AppScreen::Gameplay => {
                    return handle_gameplay_mouse_pressed_in_context(context, x, y);
                }
                AppScreen::Editor => {
                    return mutate_editor_and_render_in_context(context, |app_state, editor| {
                        editor_mouse_pressed(app_state, editor, x, y)
                    });
                }
            },
        },
        AppPointerInput::MouseReleased => {
            if matches!(active_screen(context), AppScreen::Editor) {
                let before_request = build_current_context_screen_frame_request(context);
                {
                    let runtime = context.app_runtime_mut();
                    editor_mouse_released(runtime.app_state);
                }
                let frame_changed =
                    render_current_screen_frame_if_changed(context, before_request)?;
                return Ok(current_render_work_result(context, frame_changed));
            }
        }
        AppPointerInput::Pointer { id, phase, x, y } => match current_interaction_mode(context) {
            AppInteractionMode::Gameplay => {
                return handle_gameplay_pointer_event_in_context(context, id, phase, x, y);
            }
            AppInteractionMode::Editor => {
                return mutate_editor_and_render_in_context(context, |app_state, editor| {
                    editor_touch(app_state, editor, id, phase, x, y)
                });
            }
            AppInteractionMode::Overlay(overlay) => match overlay.owning_screen() {
                AppScreen::Gameplay => {
                    return handle_gameplay_pointer_event_in_context(context, id, phase, x, y);
                }
                AppScreen::Editor => {
                    return mutate_editor_and_render_in_context(context, |app_state, editor| {
                        editor_touch(app_state, editor, id, phase, x, y)
                    });
                }
            },
        },
    }
    Ok(current_render_work_result(context, false))
}

fn current_render_work_result<C: AppDriverContext>(
    context: &mut C,
    frame_changed: bool,
) -> RenderWorkResult {
    RenderWorkResult {
        frame_changed,
        needs_followup_wake: has_pending_render_work_in_context(context),
    }
}

fn has_pending_editor_hint_job_in_context<C: AppDriverContext>(context: &mut C) -> bool {
    let Some(runtime) = context.editor_runtime_mut() else {
        return false;
    };
    runtime.app.app_state.is_editor_screen() && runtime.editor.has_active_pull_hint_job()
}

fn current_interaction_mode<C: AppDriverContext>(context: &mut C) -> AppInteractionMode {
    let runtime = context.app_runtime_mut();
    runtime.app_state.interaction_mode()
}

fn active_screen<C: AppDriverContext>(context: &mut C) -> AppScreen {
    let runtime = context.app_runtime_mut();
    runtime.app_state.active_screen()
}

fn build_current_context_screen_frame_request<C: AppDriverContext>(
    context: &mut C,
) -> Option<FrameRequest> {
    match active_screen(context) {
        AppScreen::Gameplay => {
            let runtime = context.app_runtime_mut();
            Some(build_current_gameplay_screen_frame_request(
                runtime.controller,
                runtime.app_state,
            ))
        }
        AppScreen::Editor => {
            let runtime = context.editor_runtime_mut()?;
            Some(build_current_editor_frame_request(
                runtime.app.app_state,
                runtime.editor,
            ))
        }
    }
}

fn render_current_screen_frame_if_changed<C>(
    context: &mut C,
    before_request: Option<FrameRequest>,
) -> Result<bool, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    let after_request = build_current_context_screen_frame_request(context);
    if after_request == before_request {
        return Ok(false);
    }
    let Some(request) = after_request.as_ref() else {
        return Ok(false);
    };
    context.render_frame(request)?;
    Ok(true)
}

fn mutate_editor_and_render_in_context<C, F>(
    context: &mut C,
    mutate: F,
) -> Result<RenderWorkResult, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + FrameSink<Error = <C as AppDriverContext>::Error>,
    F: FnOnce(&mut AppState, &mut LevelEditor) -> Option<EditorUiAction>,
{
    let before_request = build_current_context_screen_frame_request(context);
    let action = {
        let Some(runtime) = context.editor_runtime_mut() else {
            return Ok(RenderWorkResult::default());
        };
        mutate(runtime.app.app_state, runtime.editor)
    };
    if let Some(runtime) = context.editor_runtime_mut() {
        apply_editor_ui_action(action, runtime);
    }
    let frame_changed = render_current_screen_frame_if_changed(context, before_request)?;
    Ok(current_render_work_result(context, frame_changed))
}

fn handle_gameplay_mouse_pressed_in_context<C>(
    context: &mut C,
    x: f64,
    y: f64,
) -> Result<RenderWorkResult, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    let input = {
        let runtime = context.app_runtime_mut();
        interpret_gameplay_pointer_tap(runtime.app_state, runtime.controller, x, y)
    };
    let applied = apply_input_and_render_in_context(context, input)?;
    Ok(applied.render_work)
}

fn handle_gameplay_pointer_event_in_context<C>(
    context: &mut C,
    id: u64,
    phase: PointerPhase,
    x: f64,
    y: f64,
) -> Result<RenderWorkResult, <C as AppDriverContext>::Error>
where
    C: AppDriverContext + FrameSink<Error = <C as AppDriverContext>::Error>,
{
    let input = {
        let runtime = context.app_runtime_mut();
        interpret_gameplay_pointer_event(runtime.app_state, runtime.controller, id, phase, x, y)
    };
    let applied = apply_input_and_render_in_context(context, input)?;
    Ok(applied.render_work)
}

#[cfg(test)]
mod tests {
    use super::{
        AppDriverContext, AppFramePresenter, AppPointerInput, AppRuntimeMut,
        SharedAppRendererConfig, SharedAppRuntime, SharedAppRuntimeConfig,
        apply_action_and_render_in_context, apply_action_in_context, apply_editor_ui_action,
        apply_input_and_render_in_context, continue_pending_render_work_and_render_in_context,
        handle_pointer_input_and_render_in_context, has_pending_render_work_in_context,
    };
    use crate::app::action::AppAction;
    use crate::app::input::AppInput;
    use crate::app::presentation::{
        FrameDamage, FrameRequest, GameplayAnimationPolicy, RendererOverrides,
    };
    use crate::app::state::{AppInteractionMode, AppOverlay, AppScreen, AppState};
    use crate::editor::{EditorUiAction, build_current_editor_frame_request, editor_mouse_pressed};
    use crate::level_bootstrap::{
        InitialLevels, build_preview_boards, load_initial_levels_for_app,
    };
    use crate::persistence::{LevelPersistence, LevelSetCatalogEntry, LevelSetKind};
    use crate::shared::PointerPhase;
    use presentation::layout::{
        overlay_primary_action_button_rect, overlay_secondary_action_button_rect,
    };
    use sokobanitron_gameplay::{BoardCell, GameplayController};
    use sokobanitron_level_editor::{
        DrawTool, EditorCommand, EditorMode, ExportPuzzleError, LevelEditor,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct RecordingPresenter {
        frames: Vec<(FrameDamage, usize, u32, u32)>,
    }

    impl AppFramePresenter for RecordingPresenter {
        type Error = std::convert::Infallible;

        fn present_frame(
            &mut self,
            damage: FrameDamage,
            gray_frame: &[u8],
            width: u32,
            height: u32,
        ) -> Result<(), Self::Error> {
            self.frames.push((damage, gray_frame.len(), width, height));
            Ok(())
        }
    }

    struct TestContext {
        controller: GameplayController,
        app_state: AppState,
        level_persistence: LevelPersistence,
        preview_boards: Vec<sokobanitron_gameplay::BoardView>,
        editor: LevelEditor,
        pending_frame_presentation: bool,
        frame_followup_request: Option<FrameRequest>,
        simulate_frame_followup_on_render: bool,
        rendered_frames: Vec<FrameRequest>,
        temp_root: Option<PathBuf>,
    }

    impl TestContext {
        fn new() -> Self {
            let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
            let levels = vec![level.clone(), level.clone()];
            Self {
                controller: GameplayController::new(levels.clone(), None),
                app_state: AppState::default(),
                level_persistence: LevelPersistence::default(),
                preview_boards: build_preview_boards(&levels),
                editor: LevelEditor::new(),
                pending_frame_presentation: false,
                frame_followup_request: None,
                simulate_frame_followup_on_render: false,
                rendered_frames: Vec::new(),
                temp_root: None,
            }
        }

        fn with_pending_frame_followup() -> Self {
            let mut context = Self::new();
            context.simulate_frame_followup_on_render = true;
            context
        }

        fn with_empty_persistent_store() -> Self {
            let root = temp_dir("app-driver-editor-save-empty-store");
            let level = "#####\n#@$.#\n#####".to_string();
            Self {
                controller: GameplayController::new(vec![level.clone()], None),
                app_state: AppState::default(),
                level_persistence: LevelPersistence::bootstrap(
                    &root,
                    sokobanitron_gameplay::OrientationPolicy::Keep,
                )
                .expect("bootstrap empty persistence")
                .persistence,
                preview_boards: build_preview_boards(&[level]),
                editor: saveable_editor(),
                pending_frame_presentation: false,
                frame_followup_request: None,
                simulate_frame_followup_on_render: false,
                rendered_frames: Vec::new(),
                temp_root: Some(root),
            }
        }

        fn with_imported_level_sets() -> Self {
            let root = temp_dir("app-driver-select-level-set");
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

            let initial_levels = load_initial_levels_for_app(&root).expect("load initial levels");
            let controller = GameplayController::new_at_level(
                initial_levels.levels.clone(),
                initial_levels.initial_level_index,
                initial_levels.persisted_resume_level_index,
            );
            let mut app_state = AppState::default();
            crate::gameplay::set_gameplay_level_sets(
                &mut app_state.gameplay,
                initial_levels.level_set_catalog,
                Some(initial_levels.active_level_set_index),
            );

            Self {
                controller,
                app_state,
                level_persistence: initial_levels.persistence,
                preview_boards: initial_levels.preview_boards,
                editor: LevelEditor::new(),
                pending_frame_presentation: false,
                frame_followup_request: None,
                simulate_frame_followup_on_render: false,
                rendered_frames: Vec::new(),
                temp_root: Some(root),
            }
        }

        fn with_active_my_puzzles() -> Self {
            let root = temp_dir("app-driver-editor-save-active-my-puzzles");
            let mut level_persistence =
                LevelPersistence::bootstrap(&root, sokobanitron_gameplay::OrientationPolicy::Keep)
                    .expect("bootstrap persistence")
                    .persistence;
            level_persistence
                .save_created_puzzle("My Puzzles", "#####\n#@$.#\n#####", &[vec![(1, 2), (1, 3)]])
                .expect("seed my puzzles");
            let loaded = level_persistence
                .switch_to_level_set(0)
                .expect("switch to my puzzles")
                .expect("load my puzzles");
            let controller = GameplayController::new_at_level(
                loaded.levels.clone(),
                loaded.initial_level_index,
                loaded.persisted_resume_level_index,
            );
            let preview_boards = build_preview_boards(&loaded.levels);
            let mut app_state = AppState::default();
            crate::gameplay::set_gameplay_level_sets(
                &mut app_state.gameplay,
                level_persistence.level_set_catalog(),
                Some(
                    level_persistence
                        .active_level_set_index()
                        .expect("active set index"),
                ),
            );

            Self {
                controller,
                app_state,
                level_persistence,
                preview_boards,
                editor: saveable_editor(),
                pending_frame_presentation: false,
                frame_followup_request: None,
                simulate_frame_followup_on_render: false,
                rendered_frames: Vec::new(),
                temp_root: Some(root),
            }
        }

        fn editor_runtime_mut(&mut self) -> super::EditorAppRuntimeMut<'_> {
            AppRuntimeMut {
                controller: &mut self.controller,
                app_state: &mut self.app_state,
                level_persistence: &mut self.level_persistence,
                preview_boards: &mut self.preview_boards,
            }
            .with_editor(&mut self.editor)
        }
    }

    impl Drop for TestContext {
        fn drop(&mut self) {
            if let Some(root) = self.temp_root.take() {
                let _ = fs::remove_dir_all(root);
            }
        }
    }

    impl AppDriverContext for TestContext {
        type Error = std::convert::Infallible;

        fn app_runtime_mut(&mut self) -> super::AppRuntimeMut<'_> {
            super::AppRuntimeMut {
                controller: &mut self.controller,
                app_state: &mut self.app_state,
                level_persistence: &mut self.level_persistence,
                preview_boards: &mut self.preview_boards,
            }
        }

        fn editor_runtime_mut(&mut self) -> Option<super::EditorAppRuntimeMut<'_>> {
            Some(
                super::AppRuntimeMut {
                    controller: &mut self.controller,
                    app_state: &mut self.app_state,
                    level_persistence: &mut self.level_persistence,
                    preview_boards: &mut self.preview_boards,
                }
                .with_editor(&mut self.editor),
            )
        }

        fn has_pending_frame_presentation(&mut self) -> bool {
            self.pending_frame_presentation
        }

        fn continue_frame_presentation_and_render(&mut self) -> Result<bool, Self::Error> {
            if !self.pending_frame_presentation {
                return Ok(false);
            }
            self.pending_frame_presentation = false;
            if let Some(request) = self.frame_followup_request.take() {
                self.rendered_frames.push(request);
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    impl crate::app::presentation::FrameSink for TestContext {
        type Error = std::convert::Infallible;

        fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error> {
            self.rendered_frames.push(request.clone());
            if self.simulate_frame_followup_on_render
                && matches!(request, FrameRequest::Gameplay { .. })
            {
                self.pending_frame_presentation = true;
                self.frame_followup_request = Some(request.clone());
                self.simulate_frame_followup_on_render = false;
            }
            Ok(())
        }
    }

    fn runtime_initial_levels(levels: Vec<String>) -> InitialLevels {
        InitialLevels {
            preview_boards: build_preview_boards(&levels),
            levels,
            initial_level_index: 0,
            persisted_resume_level_index: None,
            persistence: LevelPersistence::default(),
            level_set_catalog: vec![LevelSetCatalogEntry {
                kind: LevelSetKind::Imported,
                title: "Test Levels".to_string(),
                completed_puzzle_count: 0,
                total_puzzle_count: 1,
            }],
            active_level_set_index: 0,
        }
    }

    #[test]
    fn shared_app_runtime_owns_model_and_frame_state() {
        let mut runtime = SharedAppRuntime::new(
            runtime_initial_levels(vec!["###\n#@#\n###".to_string()]),
            128,
            96,
            SharedAppRuntimeConfig::default(),
        );
        let request = runtime.current_frame_request();

        assert_eq!(runtime.gray_frame().len(), 128 * 96);
        assert!(matches!(request, FrameRequest::Gameplay { .. }));
        assert_eq!(runtime.draw_frame_request(&request), FrameDamage::Full);

        runtime.resize_surface(64, 32);

        assert_eq!(runtime.surface_width(), 64);
        assert_eq!(runtime.surface_height(), 32);
        assert_eq!(runtime.gray_frame().len(), 64 * 32);
    }

    #[test]
    fn runtime_renderer_config_is_applied_to_rendering() {
        let mut default_runtime = SharedAppRuntime::new(
            runtime_initial_levels(vec!["#####\n#@$.#\n#####".to_string()]),
            128,
            96,
            SharedAppRuntimeConfig::default(),
        );
        let mut custom_runtime = SharedAppRuntime::new(
            runtime_initial_levels(vec!["#####\n#@$.#\n#####".to_string()]),
            128,
            96,
            SharedAppRuntimeConfig {
                renderer: SharedAppRendererConfig {
                    renderer_overrides: RendererOverrides {
                        gray_1: Some(1),
                        gray_2: Some(2),
                        gray_3: Some(3),
                        gray_4: Some(4),
                        gray_5: Some(5),
                        gray_6: Some(6),
                        gray_7: Some(7),
                        gray_8: Some(8),
                        gray_9: Some(9),
                        gray_10: Some(10),
                        gray_11: Some(11),
                        gray_12: Some(12),
                        gray_13: Some(13),
                        gray_14: Some(14),
                    },
                    gameplay_animation_policy: GameplayAnimationPolicy::Limited,
                },
                ..SharedAppRuntimeConfig::default()
            },
        );
        let default_request = default_runtime.current_frame_request();
        let custom_request = custom_runtime.current_frame_request();

        default_runtime.draw_frame_request(&default_request);
        custom_runtime.draw_frame_request(&custom_request);

        assert_ne!(default_runtime.gray_frame(), custom_runtime.gray_frame());
    }

    #[test]
    fn shared_app_runtime_builds_state_from_generic_config() {
        let runtime = SharedAppRuntime::new(
            runtime_initial_levels(vec!["###\n#@#\n###".to_string()]),
            128,
            96,
            SharedAppRuntimeConfig {
                editor_available: true,
                supports_multi_touch: true,
                gameplay_touch_slop_px: Some(17),
                gameplay_max_cell_size: Some(123),
                editor_touch_slop_px: Some(19),
                editor_double_tap_window: Some(Duration::from_millis(640)),
                renderer: SharedAppRendererConfig::default(),
            },
        );

        assert!(runtime.app_state.supports_multi_touch);
        assert!(runtime.app_state.editor_available);
        assert_eq!(runtime.app_state.gameplay.max_cell_size, 123);
        assert_eq!(
            runtime.app_state.editor.interaction.double_tap_window,
            Duration::from_millis(640)
        );
    }

    #[test]
    fn shared_app_runtime_applies_input_and_outputs_rendered_frames() {
        let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
        let mut runtime = SharedAppRuntime::new(
            runtime_initial_levels(vec![level.clone(), level]),
            128,
            96,
            SharedAppRuntimeConfig::default(),
        );
        let mut presenter = RecordingPresenter::default();

        let applied = runtime
            .apply_input_and_render(AppInput::BoardTap(BoardCell::new(1, 1)), &mut presenter)
            .unwrap();

        assert!(applied.rendered_frame);
        assert!(!presenter.frames.is_empty());
        assert_eq!(presenter.frames[0].1, 128 * 96);
        assert_eq!(presenter.frames[0].2, 128);
        assert_eq!(presenter.frames[0].3, 96);
    }

    #[test]
    fn no_op_action_has_no_presentation_calls() {
        let mut context = TestContext::new();

        let applied = apply_action_in_context(&mut context, AppAction::NoOp).unwrap();

        assert_eq!(applied.changes, Default::default());
        assert!(applied.presentation_plan.is_none());
        assert!(!applied.rendered_frame);
    }

    #[test]
    fn board_tap_action_returns_presentation_plan() {
        let mut context = TestContext::new();

        let applied =
            apply_action_in_context(&mut context, AppAction::TapBoardCell(BoardCell::new(1, 1)))
                .unwrap();

        assert!(applied.presentation_plan.is_some());
        assert!(!applied.rendered_frame);
    }

    #[test]
    fn apply_action_and_render_advances_after_solved_and_renders_new_level() {
        let mut context = TestContext::new();

        let applied =
            apply_action_and_render_in_context(&mut context, AppAction::AdvanceAfterSolved)
                .unwrap();

        assert_eq!(applied.persistence.resume_level_to_persist, Some(1));
        assert!(applied.presentation_plan.is_some());
        assert!(applied.rendered_frame);
        assert_eq!(context.rendered_frames.len(), 1);
        let FrameRequest::Gameplay { update, .. } = &context.rendered_frames[0] else {
            panic!("expected gameplay frame");
        };
        assert_eq!(
            update.cause,
            presentation::screen_requests::GameplayPresentationCause::CurrentState
        );
        assert_eq!(update.scene.level_number, 2);
    }

    #[test]
    fn apply_input_and_render_interprets_before_applying() {
        let mut context = TestContext::new();

        let applied = apply_input_and_render_in_context(
            &mut context,
            AppInput::BoardTap(BoardCell::new(1, 1)),
        )
        .unwrap();

        assert!(applied.presentation_plan.is_some());
        assert!(applied.rendered_frame);
    }

    #[test]
    fn enter_editor_mode_renders_from_shared_driver() {
        let mut context = TestContext::new();

        let applied =
            apply_action_and_render_in_context(&mut context, AppAction::EnterEditorMode).unwrap();

        assert!(context.app_state.is_editor_screen());
        assert!(applied.presentation_plan.is_none());
        assert!(applied.rendered_frame);
        assert_eq!(context.rendered_frames.len(), 1);
        let FrameRequest::Editor { .. } = &context.rendered_frames[0] else {
            panic!("expected editor frame");
        };
    }

    #[test]
    fn open_level_select_renders_current_overlay_frame_from_shared_driver() {
        let mut context = TestContext::new();

        let applied =
            apply_input_and_render_in_context(&mut context, AppInput::OpenLevelSelect).unwrap();

        assert!(applied.presentation_plan.is_none());
        assert!(applied.rendered_frame);
        assert_eq!(context.rendered_frames.len(), 1);
        let FrameRequest::LevelSelect { .. } = &context.rendered_frames[0] else {
            panic!("expected level select frame");
        };
    }

    #[test]
    fn open_gameplay_overlay_renders_from_shared_driver() {
        let mut context = TestContext::new();

        let applied =
            apply_input_and_render_in_context(&mut context, AppInput::OverlayOpen).unwrap();

        assert!(applied.presentation_plan.is_none());
        assert!(applied.rendered_frame);
        assert_eq!(context.rendered_frames.len(), 1);
        let FrameRequest::GameplayMenu { .. } = &context.rendered_frames[0] else {
            panic!("expected gameplay menu frame");
        };
    }

    #[test]
    fn apply_action_and_render_executes_presentation_plan_when_present() {
        let mut context = TestContext::new();

        let applied = apply_action_and_render_in_context(
            &mut context,
            AppAction::TapBoardCell(BoardCell::new(1, 1)),
        )
        .unwrap();

        assert!(applied.presentation_plan.is_some());
        assert!(applied.rendered_frame);
        assert_eq!(context.rendered_frames.len(), 1);
    }

    #[test]
    fn apply_action_and_render_reports_pending_frame_followup_work() {
        let mut context = TestContext::with_pending_frame_followup();

        let applied = apply_action_and_render_in_context(
            &mut context,
            AppAction::TapBoardCell(BoardCell::new(1, 1)),
        )
        .unwrap();

        assert!(applied.rendered_frame);
        assert!(applied.render_work.needs_followup_wake);
        assert!(has_pending_render_work_in_context(&mut context));
    }

    #[test]
    fn continue_pending_render_work_advances_frame_followup_from_shared_driver() {
        let mut context = TestContext::with_pending_frame_followup();
        let _ = apply_action_and_render_in_context(
            &mut context,
            AppAction::TapBoardCell(BoardCell::new(1, 1)),
        )
        .unwrap();
        assert_eq!(context.rendered_frames.len(), 1);

        let work = continue_pending_render_work_and_render_in_context(&mut context).unwrap();

        assert!(work.frame_changed);
        assert!(!work.needs_followup_wake);
        assert_eq!(context.rendered_frames.len(), 2);
        let FrameRequest::Gameplay { .. } = &context.rendered_frames[1] else {
            panic!("expected gameplay frame");
        };
    }

    #[test]
    fn selecting_level_set_activates_it_before_rendering_current_gameplay_frame() {
        let mut context = TestContext::with_imported_level_sets();
        context.app_state.ui.overlay =
            Some(crate::app::state::AppOverlay::LevelSetSelect { page_start: 0 });

        let applied =
            apply_action_and_render_in_context(&mut context, AppAction::SelectLevelSet(1)).unwrap();

        assert_eq!(applied.level_set_selected, Some(1));
        assert!(applied.rendered_frame);
        assert_eq!(context.app_state.gameplay.active_level_set, Some(1));
        assert_eq!(context.controller.level_count(), 2);
        assert_eq!(context.preview_boards.len(), 2);
        assert_eq!(context.rendered_frames.len(), 1);
        let FrameRequest::Gameplay { update, .. } = &context.rendered_frames[0] else {
            panic!("expected gameplay frame");
        };
        assert_eq!(update.scene.board, context.controller.board().clone());
    }

    #[test]
    fn save_editor_action_updates_level_set_catalog() {
        let mut context = TestContext::with_empty_persistent_store();
        let runtime = context.editor_runtime_mut();

        apply_editor_ui_action(Some(EditorUiAction::SavePuzzle), runtime);

        assert_eq!(context.app_state.gameplay.level_sets.len(), 1);
        assert_eq!(context.app_state.gameplay.level_sets[0].title, "My Puzzles");
        assert_eq!(
            context.app_state.gameplay.level_sets[0].total_puzzle_count,
            1
        );
    }

    #[test]
    fn save_editor_action_refreshes_active_my_puzzles_runtime() {
        let mut context = TestContext::with_active_my_puzzles();
        let runtime = context.editor_runtime_mut();

        apply_editor_ui_action(Some(EditorUiAction::SavePuzzle), runtime);

        assert_eq!(context.app_state.gameplay.active_level_set, Some(0));
        assert_eq!(context.controller.level_count(), 2);
        assert_eq!(context.preview_boards.len(), 2);
    }

    #[test]
    fn save_editor_action_resets_editor_after_success() {
        let mut context = TestContext::with_empty_persistent_store();
        let runtime = context.editor_runtime_mut();

        apply_editor_ui_action(Some(EditorUiAction::SavePuzzle), runtime);

        assert!(context.editor.world().box_positions().is_empty());
        assert_eq!(
            context.editor.export_puzzle(),
            Err(ExportPuzzleError::MissingPlayer)
        );
    }

    #[test]
    fn save_editor_action_resets_editor_interaction_state() {
        let mut context = TestContext::with_empty_persistent_store();
        context.app_state.ui.screen = crate::app::state::AppScreen::Editor;
        let _ = editor_mouse_pressed(&mut context.app_state, &mut context.editor, 100.0, 100.0);
        assert!(
            context
                .app_state
                .editor
                .interaction
                .touch
                .active_position()
                .is_some()
        );
        let runtime = context.editor_runtime_mut();

        apply_editor_ui_action(Some(EditorUiAction::SavePuzzle), runtime);

        assert!(
            context
                .app_state
                .editor
                .interaction
                .touch
                .active_position()
                .is_none()
        );
    }

    #[test]
    fn no_editor_action_is_noop() {
        let mut context = TestContext::with_empty_persistent_store();
        let original_level_count = context.controller.level_count();
        let runtime = context.editor_runtime_mut();

        apply_editor_ui_action(None, runtime);

        assert_eq!(context.controller.level_count(), original_level_count);
        assert_eq!(context.app_state.gameplay.level_sets.len(), 0);
    }

    #[test]
    fn mouse_press_routes_editor_overlay_primary_action_through_shared_handler() {
        let mut context = TestContext::new();
        context.app_state.ui.screen = AppScreen::Editor;
        context.app_state.ui.overlay = Some(AppOverlay::EditorMenu);
        assert_eq!(
            context.app_state.interaction_mode(),
            AppInteractionMode::Overlay(AppOverlay::EditorMenu)
        );
        let rect = overlay_primary_action_button_rect(
            context.app_state.editor.viewport.surface_width,
            context.app_state.editor.viewport.surface_height,
        );

        handle_pointer_input_and_render_in_context(
            &mut context,
            AppPointerInput::MousePressed {
                x: rect_center_x(rect),
                y: rect_center_y(rect),
            },
        )
        .unwrap();

        assert!(context.app_state.is_gameplay_screen());
        assert_eq!(context.app_state.ui.overlay, None);
        assert_eq!(context.rendered_frames.len(), 1);
        let FrameRequest::Gameplay { .. } = &context.rendered_frames[0] else {
            panic!("expected gameplay frame");
        };
    }

    #[test]
    fn pointer_routes_editor_overlay_secondary_action_through_shared_handler() {
        let mut context = TestContext::with_empty_persistent_store();
        context.app_state.ui.screen = AppScreen::Editor;
        context.app_state.ui.overlay = Some(AppOverlay::EditorMenu);
        assert_eq!(
            context.app_state.interaction_mode(),
            AppInteractionMode::Overlay(AppOverlay::EditorMenu)
        );
        let rect = overlay_secondary_action_button_rect(
            context.app_state.editor.viewport.surface_width,
            context.app_state.editor.viewport.surface_height,
        );

        handle_pointer_input_and_render_in_context(
            &mut context,
            AppPointerInput::Pointer {
                id: 7,
                phase: PointerPhase::Started,
                x: rect_center_x(rect),
                y: rect_center_y(rect),
            },
        )
        .unwrap();

        handle_pointer_input_and_render_in_context(
            &mut context,
            AppPointerInput::Pointer {
                id: 7,
                phase: PointerPhase::Ended,
                x: rect_center_x(rect),
                y: rect_center_y(rect),
            },
        )
        .unwrap();

        assert!(context.app_state.is_editor_screen());
        assert_eq!(context.app_state.ui.overlay, None);
        assert_eq!(context.app_state.gameplay.level_sets.len(), 1);
        assert_eq!(
            context.editor.export_puzzle(),
            Err(ExportPuzzleError::MissingPlayer)
        );
        assert_eq!(context.rendered_frames.len(), 1);
        let FrameRequest::Editor { .. } = &context.rendered_frames[0] else {
            panic!("expected editor frame");
        };
    }

    #[test]
    fn editor_mouse_released_through_shared_handler_clears_interaction_without_render() {
        let mut context = TestContext::new();
        context.app_state.ui.screen = AppScreen::Editor;
        let (press_x, press_y) = editor_board_cell_center(&context);

        let _ = editor_mouse_pressed(
            &mut context.app_state,
            &mut context.editor,
            press_x,
            press_y,
        );

        assert!(
            context
                .app_state
                .editor
                .interaction
                .touch
                .active_position()
                .is_some()
        );
        assert!(context.app_state.editor.interaction.active_stroke.is_some());

        handle_pointer_input_and_render_in_context(&mut context, AppPointerInput::MouseReleased)
            .unwrap();

        assert!(
            context
                .app_state
                .editor
                .interaction
                .touch
                .active_position()
                .is_none()
        );
        assert!(context.app_state.editor.interaction.active_stroke.is_none());
        assert!(context.rendered_frames.is_empty());
    }

    fn saveable_editor() -> LevelEditor {
        let mut editor = LevelEditor::new();
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 2,
            cell_y: 0,
            tool: DrawTool::Floor,
        });
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 0,
            cell_y: 0,
            tool: DrawTool::GoalWithBox,
        });
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });
        editor
    }

    fn editor_board_cell_center(context: &TestContext) -> (f64, f64) {
        let FrameRequest::Editor { screen } =
            build_current_editor_frame_request(&context.app_state, &context.editor)
        else {
            panic!("expected editor frame");
        };
        let cell = BoardCell::new(screen.board.width() / 2, screen.board.height() / 2);
        let (x, y, w, h) = screen.viewport.cell_to_screen_rect(cell);
        (x as f64 + (w as f64 / 2.0), y as f64 + (h as f64 / 2.0))
    }

    fn rect_center_x(rect: presentation::layout::ScreenRect) -> f64 {
        rect.x as f64 + (rect.w as f64 / 2.0)
    }

    fn rect_center_y(rect: presentation::layout::ScreenRect) -> f64 {
        rect.y as f64 + (rect.h as f64 / 2.0)
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let unique = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("sokobanitron-{name}-{nanos}-{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
