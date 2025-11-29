use std::fs::File;
use std::io::Write;
use std::process::Command;

mod audio;
mod scene;

// Configuration for our animation
const FRAMES: i32 = 1800; // 30 seconds at 60 FPS
const OUTPUT_FILENAME: &str = "generated_script.py";
const RENDER_OUTPUT: &str = "//render_output"; // Blender relative path

fn main() -> std::io::Result<()> {
    println!("Generating audio...");
    audio::generate_audio("audio.wav", 30)?; // 30 seconds of audio
    println!("Audio generated: audio.wav");

    println!("🦀 Rust is calculating animation data...");

    let mut script = String::from(r#"
import bpy
import math

# --- Setup Scene ---
bpy.ops.object.select_all(action='DESELECT')
bpy.ops.object.select_by_type(type='MESH')
bpy.ops.object.delete()

# Set end frame and FPS
bpy.context.scene.render.fps = 60
bpy.context.scene.frame_end = "#);
    script.push_str(&format!("{}\n", FRAMES));

    // --- Materials ---
    script.push_str(r#"
def create_material(name, color, emission_strength=0):
    mat = bpy.data.materials.new(name=name)
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    bsdf = nodes.get("Principled BSDF")
    bsdf.inputs['Base Color'].default_value = color
    if emission_strength > 0:
        bsdf.inputs['Emission'].default_value = color
        bsdf.inputs['Emission Strength'].default_value = emission_strength
    return mat

mat_blue = create_material("NeonBlue", (0, 0.5, 1, 1), 2.0)
mat_orange = create_material("NeonOrange", (1, 0.2, 0, 1), 2.0)
mat_skin = create_material("Skin", (1, 0.8, 0.6, 1), 0.0)
mat_dark = create_material("DarkVoid", (0.05, 0.05, 0.05, 1), 0.0)
mat_grid = create_material("Grid", (0, 1, 0.8, 1), 5.0)
"#);

    // --- Environment ---
    // Road
    script.push_str(r#"
bpy.ops.mesh.primitive_plane_add(size=100, location=(0, 0, 0))
road = bpy.context.active_object
road.name = "Road"
road.scale = (0.1, 10, 1) # Long strip
road.data.materials.append(mat_dark)

# Grid Lines (Procedural)
for i in range(-20, 20):
    bpy.ops.mesh.primitive_cube_add(size=0.1, location=(i * 2, 0, -0.1))
    line = bpy.context.active_object
    line.scale = (0.5, 1000, 0.5)
    line.data.materials.append(mat_grid)
"#);

    // --- Character Setup ---
    // We create the objects once, then animate them
    // Use the first frame to define initial positions
    let initial_objects = scene::calculate_walk_cycle(0, FRAMES);
    
    for obj in &initial_objects {
        script.push_str(&format!(
            "bpy.ops.mesh.primitive_cube_add(size=1, location=({:.4}, {:.4}, {:.4}))\n",
            obj.location.x, obj.location.y, obj.location.z
        ));
        script.push_str("obj = bpy.context.active_object\n");
        script.push_str(&format!("obj.name = '{}'\n", obj.name));
        script.push_str(&format!("obj.scale = ({:.4}, {:.4}, {:.4})\n", obj.scale.x, obj.scale.y, obj.scale.z));
        script.push_str(&format!("obj.rotation_euler = ({:.4}, {:.4}, {:.4})\n", obj.rotation.x, obj.rotation.y, obj.rotation.z));
        
        // Assign Material based on name
        if obj.name.contains("Head") || obj.name.contains("Arm") || obj.name.contains("Leg") {
             script.push_str("obj.data.materials.append(mat_skin if 'Head' in obj.name else mat_blue)\n");
        } else {
             script.push_str("obj.data.materials.append(mat_orange)\n");
        }
    }

    // Parenting (must be done after all objects created)
    for obj in &initial_objects {
        if let Some(parent_name) = &obj.parent {
            script.push_str(&format!("bpy.data.objects['{}'].parent = bpy.data.objects['{}']\n", obj.name, parent_name));
        }
    }

    // --- Animation Loop ---
    for frame in 0..=FRAMES {
        let objects = scene::calculate_walk_cycle(frame, FRAMES);
        
        // Move the character forward along Y axis
        let forward_speed = 0.1;
        let y_offset = frame as f32 * forward_speed;

        for obj in objects {
            // We only need to update location/rotation relative to parent or world
            // Since we parented, local coordinates work best.
            // However, our calculate_walk_cycle returns local coords for limbs but world-ish for Torso.
            // Let's just update Torso world position and Limbs local rotation/position.
            
            if obj.parent.is_none() {
                // Root object (Torso) moves in world
                script.push_str(&format!("obj = bpy.data.objects['{}']\n", obj.name));
                script.push_str(&format!("obj.location = ({:.4}, {:.4}, {:.4})\n", obj.location.x, obj.location.y - y_offset, obj.location.z));
                script.push_str(&format!("obj.rotation_euler = ({:.4}, {:.4}, {:.4})\n", obj.rotation.x, obj.rotation.y, obj.rotation.z));
                script.push_str(&format!("obj.keyframe_insert(data_path='location', frame={})\n", frame));
                script.push_str(&format!("obj.keyframe_insert(data_path='rotation_euler', frame={})\n", frame));
            } else {
                // Child objects (Limbs) - update local transform
                script.push_str(&format!("obj = bpy.data.objects['{}']\n", obj.name));
                script.push_str(&format!("obj.location = ({:.4}, {:.4}, {:.4})\n", obj.location.x, obj.location.y, obj.location.z));
                script.push_str(&format!("obj.rotation_euler = ({:.4}, {:.4}, {:.4})\n", obj.rotation.x, obj.rotation.y, obj.rotation.z));
                script.push_str(&format!("obj.keyframe_insert(data_path='location', frame={})\n", frame));
                script.push_str(&format!("obj.keyframe_insert(data_path='rotation_euler', frame={})\n", frame));
            }
        }
    }

    // --- Camera & Audio ---
    script.push_str(r#"
# Camera Setup
camera_data = bpy.data.cameras.new(name='Camera')
camera_object = bpy.data.objects.new('Camera', camera_data)
bpy.context.collection.objects.link(camera_object)
bpy.context.scene.camera = camera_object

# Camera constraint to follow Torso
const = camera_object.constraints.new(type='TRACK_TO')
const.target = bpy.data.objects['Torso']
const.track_axis = 'TRACK_NEGATIVE_Z'
const.up_axis = 'UP_Y'

# Animate Camera following
for frame in range(0, "#);
    script.push_str(&format!("{}", FRAMES + 1));
    script.push_str(r#"):
    y_pos = -(frame * 0.1) + 8 # Keep distance
    camera_object.location = (5, y_pos, 3)
    camera_object.keyframe_insert(data_path='location', frame=frame)

# Audio Setup (VSE)
if not bpy.context.scene.sequence_editor:
    bpy.context.scene.sequence_editor_create()

seq = bpy.context.scene.sequence_editor.sequences.new_sound(
    name="Beat",
    filepath="audio.wav",
    channel=1,
    frame_start=1
)

# Render Settings
bpy.context.scene.render.engine = 'BLENDER_EEVEE'
bpy.context.scene.eevee.use_bloom = True # Enable Bloom for Neon
bpy.context.scene.render.image_settings.file_format = 'FFMPEG'
bpy.context.scene.render.ffmpeg.format = 'MPEG4'
bpy.context.scene.render.ffmpeg.codec = 'H264'
bpy.context.scene.render.ffmpeg.audio_codec = 'AAC'
bpy.context.scene.render.filepath = '"#);

    script.push_str(RENDER_OUTPUT);
    script.push_str("'\n");

    let mut file = File::create(OUTPUT_FILENAME)?;
    file.write_all(script.as_bytes())?;
    
    println!("✅ Python script generated successfully.");
    println!("🎥 Launching Blender to render video...");

    // ... (Blender execution code remains similar but we need to ensure audio.wav is found) ...
    // For brevity, I'll assume the existing Blender finding code is fine, 
    // but I need to make sure I don't delete it or I rewrite it.
    // The ReplacementContent above ends before the Blender execution part.
    // Wait, I need to check where I cut off.
    // I replaced from `fn main() ...` to the end of the file? 
    // No, I should check the EndLine. 
    // The previous file had 146 lines.
    // I should probably rewrite the whole main function to be safe.
    
    // Re-adding the Blender execution part to the ReplacementContent
    let blender_paths = vec![
        "blender",
        "/Applications/Blender.app/Contents/MacOS/Blender",
        "/usr/bin/blender",
        "C:\\Program Files\\Blender Foundation\\Blender 3.6\\blender.exe",
    ];

    let mut blender_cmd = None;
    for path in &blender_paths {
        if Command::new(path).arg("--version").output().is_ok() {
            blender_cmd = Some(path.to_string());
            println!("📍 Found Blender at: {}", path);
            break;
        }
    }

    match blender_cmd {
        Some(blender) => {
            let output = Command::new(blender)
                .arg("-b")
                .arg("-P")
                .arg(OUTPUT_FILENAME)
                .arg("-a") // -noaudio is REMOVED
                .output();

            match output {
                Ok(o) => {
                    if o.status.success() {
                        println!("🚀 Rendering Complete! Check the folder for render_output.mp4");
                    } else {
                        eprintln!("Error during rendering: {}", String::from_utf8_lossy(&o.stderr));
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute Blender.");
                    eprintln!("Error: {}", e);
                }
            }
        }
        None => {
            eprintln!("❌ Failed to find Blender.");
            eprintln!("You can manually run: blender -b -P {} -a", OUTPUT_FILENAME);
        }
    }

    Ok(())
}
