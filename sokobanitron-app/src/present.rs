use crate::frame::FrameRequest;
use crate::presentation_profile::{
    BoxPathStyle, BoxRemovedStyle, PresentMode, PresentationProfile,
};
use sokobanitron_gameplay::{GameplayTapEffect, GameplayTapOutcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentationStep {
    Render(FrameRequest),
    AnimatePlayerBlink,
    AnimateBoxVanish {
        to_x: u32,
        to_y: u32,
        show_solved_overlay: bool,
    },
    AnimateBoxPathDisappear {
        path: Vec<(u32, u32)>,
        show_solved_overlay: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PresentationPlan {
    pub steps: Vec<PresentationStep>,
}

pub trait FrameSink {
    type Error;

    fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error>;
    fn animate_player_blink(&mut self) -> Result<(), Self::Error>;
    fn animate_box_vanish(
        &mut self,
        to_x: u32,
        to_y: u32,
        show_solved_overlay: bool,
    ) -> Result<(), Self::Error>;
    fn animate_box_path_disappear(
        &mut self,
        path: &[(u32, u32)],
        show_solved_overlay: bool,
    ) -> Result<(), Self::Error>;
}

fn gameplay_render_step(
    box_trail: Option<Vec<(u32, u32)>>,
    draw_player: bool,
    show_solved_overlay: bool,
    present_mode: PresentMode,
) -> PresentationStep {
    PresentationStep::Render(FrameRequest::Gameplay {
        box_trail,
        draw_player,
        show_solved_overlay,
        present_mode,
    })
}

pub fn build_presentation_plan(
    outcome: &GameplayTapOutcome,
    profile: &PresentationProfile,
) -> PresentationPlan {
    let mut steps = Vec::new();
    let delay_solved_overlay = outcome.became_solved && profile.allow_delays;
    let show_solved_overlay_now = !delay_solved_overlay;

    if matches!(outcome.effect, GameplayTapEffect::MoveRejected) {
        steps.push(PresentationStep::AnimatePlayerBlink);
        return PresentationPlan { steps };
    }

    if let GameplayTapEffect::BoxRemoved { to_x, to_y } = outcome.effect {
        match profile.box_removed_style {
            BoxRemovedStyle::ImmediateRender => {
                steps.push(gameplay_render_step(
                    None,
                    true,
                    show_solved_overlay_now,
                    PresentMode::Full,
                ));
            }
            BoxRemovedStyle::VanishThenBlink => {
                steps.push(PresentationStep::AnimateBoxVanish {
                    to_x,
                    to_y,
                    show_solved_overlay: show_solved_overlay_now,
                });
                steps.push(PresentationStep::AnimatePlayerBlink);
            }
            BoxRemovedStyle::RenderThenBlink => {
                steps.push(gameplay_render_step(
                    None,
                    true,
                    show_solved_overlay_now,
                    PresentMode::Full,
                ));
                steps.push(PresentationStep::AnimatePlayerBlink);
            }
        }

        return PresentationPlan { steps };
    }

    if let GameplayTapEffect::BoxMoved { path } = &outcome.effect
        && path.len() > 2
    {
        match profile.box_path_style {
            BoxPathStyle::Hidden => {}
            BoxPathStyle::FlashThenHide => {
                steps.push(gameplay_render_step(
                    Some(path.clone()),
                    false,
                    show_solved_overlay_now,
                    PresentMode::Full,
                ));

                if outcome.dirty_solution {
                    steps.push(PresentationStep::AnimatePlayerBlink);
                } else {
                    let final_show_solved_overlay = if delay_solved_overlay {
                        true
                    } else {
                        show_solved_overlay_now
                    };
                    let final_present_mode = if delay_solved_overlay {
                        profile.delayed_solved_present_mode
                    } else {
                        PresentMode::Full
                    };
                    steps.push(gameplay_render_step(
                        None,
                        true,
                        final_show_solved_overlay,
                        final_present_mode,
                    ));
                }

                return PresentationPlan { steps };
            }
            BoxPathStyle::AnimatePathDisappear => {
                steps.push(PresentationStep::AnimateBoxPathDisappear {
                    path: path.clone(),
                    show_solved_overlay: show_solved_overlay_now,
                });

                if outcome.dirty_solution {
                    steps.push(PresentationStep::AnimatePlayerBlink);
                } else if delay_solved_overlay {
                    steps.push(gameplay_render_step(
                        None,
                        true,
                        true,
                        profile.delayed_solved_present_mode,
                    ));
                }

                return PresentationPlan { steps };
            }
        }
    }

    if !matches!(outcome.effect, GameplayTapEffect::None) {
        steps.push(gameplay_render_step(
            None,
            true,
            show_solved_overlay_now,
            PresentMode::Full,
        ));
    }

    if outcome.dirty_solution {
        steps.push(PresentationStep::AnimatePlayerBlink);
    } else if delay_solved_overlay {
        steps.push(gameplay_render_step(
            None,
            true,
            true,
            profile.delayed_solved_present_mode,
        ));
    }

    PresentationPlan { steps }
}

pub fn execute_presentation_plan<S: FrameSink>(
    sink: &mut S,
    plan: &PresentationPlan,
) -> Result<(), S::Error> {
    for step in &plan.steps {
        match step {
            PresentationStep::Render(request) => sink.render_frame(request)?,
            PresentationStep::AnimatePlayerBlink => sink.animate_player_blink()?,
            PresentationStep::AnimateBoxVanish {
                to_x,
                to_y,
                show_solved_overlay,
            } => sink.animate_box_vanish(*to_x, *to_y, *show_solved_overlay)?,
            PresentationStep::AnimateBoxPathDisappear {
                path,
                show_solved_overlay,
            } => sink.animate_box_path_disappear(path, *show_solved_overlay)?,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PresentationStep, build_presentation_plan};
    use crate::presentation_profile::{
        BoxPathStyle, BoxRemovedStyle, PresentMode, PresentationProfile,
    };
    use sokobanitron_gameplay::{GameplayControllerChanges, GameplayTapEffect, GameplayTapOutcome};

    const PROFILE_VANISH: PresentationProfile = PresentationProfile {
        box_removed_style: BoxRemovedStyle::VanishThenBlink,
        box_path_style: BoxPathStyle::AnimatePathDisappear,
        delayed_solved_present_mode: PresentMode::FastPartial,
        allow_delays: true,
    };

    const PROFILE_RENDER: PresentationProfile = PresentationProfile {
        box_removed_style: BoxRemovedStyle::RenderThenBlink,
        box_path_style: BoxPathStyle::AnimatePathDisappear,
        delayed_solved_present_mode: PresentMode::FastPartial,
        allow_delays: true,
    };

    fn solved_box_removed_outcome() -> GameplayTapOutcome {
        GameplayTapOutcome {
            changes: GameplayControllerChanges::default(),
            effect: GameplayTapEffect::BoxRemoved { to_x: 4, to_y: 7 },
            became_solved: true,
            dirty_solution: false,
            started_now: false,
        }
    }

    #[test]
    fn box_removed_vanish_then_blink_has_no_delayed_solved_render() {
        let outcome = solved_box_removed_outcome();
        let plan = build_presentation_plan(&outcome, &PROFILE_VANISH);

        assert_eq!(plan.steps.len(), 2);
        assert!(matches!(
            plan.steps[0],
            PresentationStep::AnimateBoxVanish {
                to_x: 4,
                to_y: 7,
                show_solved_overlay: false,
            }
        ));
        assert!(matches!(
            plan.steps[1],
            PresentationStep::AnimatePlayerBlink
        ));
    }

    #[test]
    fn box_removed_render_then_blink_has_no_delayed_solved_render() {
        let outcome = solved_box_removed_outcome();
        let plan = build_presentation_plan(&outcome, &PROFILE_RENDER);

        assert_eq!(plan.steps.len(), 2);
        assert!(matches!(
            plan.steps[0],
            PresentationStep::Render(crate::frame::FrameRequest::Gameplay {
                show_solved_overlay: false,
                ..
            })
        ));
        assert!(matches!(
            plan.steps[1],
            PresentationStep::AnimatePlayerBlink
        ));
    }
}
