//! Bad authoring data must fail during loading, before it can reach gameplay.
use jackal_native::assets::Assets;
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "jackal-assets-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        let root = Assets::default_directory();
        for sub in ["levels", "textures"] {
            fs::create_dir(dir.join(sub)).unwrap();
            for entry in fs::read_dir(root.join(sub)).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    fs::copy(entry.path(), dir.join(sub).join(entry.file_name())).unwrap();
                }
            }
        }
        for file in [
            "audio.json",
            "animations.json",
            "destruction.json",
            "texture_phases.json",
            "boss_patches.json",
        ] {
            fs::copy(root.join(file), dir.join(file)).unwrap();
        }
        fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    copy_tree(&entry.path(), &to.join(entry.file_name()));
                } else {
                    fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
                }
            }
        }
        copy_tree(&root.join("cinematics"), &dir.join("cinematics"));
        copy_tree(&root.join("fonts"), &dir.join("fonts"));
        copy_tree(&root.join("textures"), &dir.join("textures"));
        Self(dir)
    }
    fn edit(&self, path: &str, edit: impl FnOnce(&mut Value)) {
        let path = self.0.join(path);
        let mut json: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        edit(&mut json);
        fs::write(path, serde_json::to_vec(&json).unwrap()).unwrap();
    }
    fn rejects(&self, message: &str) {
        let error = Assets::load(&self.0)
            .err()
            .expect("invalid asset accepted")
            .to_string();
        assert!(error.contains(message), "unexpected error: {error}");
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn tile_size_and_missing_collision_cells_fail_before_gameplay() {
    let f = Fixture::new();
    f.edit("levels/stage_01.json", |j| j["tile_size"] = 0.into());
    f.rejects("invalid dimensions");
    f.edit("levels/stage_01.json", |j| {
        j["tile_size"] = 8.into();
        j["collision"].as_array_mut().unwrap().pop();
    });
    f.rejects("inconsistent tile layer lengths");
}
#[test]
fn invalid_animation_references_and_missing_clips_are_rejected() {
    let f = Fixture::new();
    f.edit("animations.json", |j| {
        j["jeep_north"]["frames"] = serde_json::json!([256])
    });
    f.rejects("Invalid animation");
    f.edit("animations.json", |j| {
        j.as_object_mut().unwrap().remove("jeep_north");
    });
    f.rejects("Missing animation");
}
#[test]
fn duplicate_ids_and_unknown_families_are_rejected() {
    let f = Fixture::new();
    f.edit("levels/entity_specs.json", |j| {
        j["specs"][1]["source_type"] = j["specs"][0]["source_type"].clone()
    });
    f.rejects("Duplicate entity ID");
    f.edit("levels/entity_specs.json", |j| {
        j["specs"][0]["family"] = "typo".into()
    });
    f.rejects("Unknown entity family");
}
#[test]
fn wrong_atlas_dimensions_are_rejected() {
    let f = Fixture::new();
    image::RgbaImage::new(8, 8)
        .save(f.0.join("textures/stage_01_tiles.png"))
        .unwrap();
    f.rejects("invalid atlas dimensions");
}
