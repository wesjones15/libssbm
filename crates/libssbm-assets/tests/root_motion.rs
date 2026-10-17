use glam::Vec3;
use libssbm::{Contact, Event, Host, Input, Move, Sim};
use libssbm_assets::{Result, animation::Animations, model::Model};

struct Floor {
    wall: f32,
}
impl Host for Floor {
    fn collide(&mut self, _: Vec3, desired: Vec3) -> Contact {
        Contact {
            position: Vec3::new(desired.x.min(self.wall), 0., desired.z),
            grounded: true,
            ceiling: false,
        }
    }
    fn event(&mut self, _: Event) {}
}

#[test]
fn dash_commits_disc_translation_once_without_snapback_and_respects_walls() -> Result<()> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if !dir.join("PlSs.dat").exists() {
        eprintln!("SKIP: real-disc cache absent");
        return Ok(());
    }
    let animations = Animations::load(&dir)?;
    let definitions = animations.motion_definitions();
    let dash = &definitions["AttackDash"];
    let travel = dash
        .forward_delta(0., dash.frames)
        .expect("source Dash Attack motion flag");
    assert!(travel > 10., "source dash must travel: {travel}");
    let model = Model::load(&dir, 0)?;
    let joint = animations.clips["AttackDash"].motion_joint.unwrap();
    for frame in [0., 12., dash.frames] {
        let pose = animations.pose(&model, "AttackDash", frame, false);
        let parent = model.joints[joint].parent.unwrap();
        let local = pose[parent].inverse() * pose[joint];
        assert!(
            local.w_axis.truncate().length() < 1e-4,
            "model must not add another dash"
        );
    }
    let params = libssbm_assets::params::load(&dir)?;
    let actions = libssbm_assets::action::load(&dir, &animations)?;
    for fps in [30, 60, 144] {
        for wall in [f32::INFINITY, travel * 0.4] {
            let mut sim = Sim::new(params.clone(), Vec3::ZERO);
            sim.grounded = true;
            sim.motion.definitions = definitions.clone();
            sim.action.definitions = actions.clone();
            let mut host = Floor { wall };
            assert!(sim.start_move(&mut host, Move::DashAttack, Input::idle()));
            let mut furthest = 0f32;
            for _ in 0..fps * 2 {
                sim.advance(&mut host, Input::idle(), 1. / fps as f64);
                assert!(sim.position.x >= furthest - 1e-4, "snapback at {fps} Hz");
                furthest = furthest.max(sim.position.x);
            }
            assert_eq!(sim.current_move, None);
            assert!(
                (sim.position.x - travel.min(wall)).abs() < 0.001,
                "distance lost/doubled at {fps} Hz: {} vs {}",
                sim.position.x,
                travel.min(wall)
            );
        }
    }
    Ok(())
}
