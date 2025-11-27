use std::fs::File;
use std::io::Write;
use std::process::Command;

// Configuration for our animation
const NUM_CUBES: i32 = 10;
const FRAMES: i32 = 60;
const OUTPUT_FILENAME: &str = "generated_script.py";
const RENDER_OUTPUT: &str = "//render_output"; // Blender relative path

fn main() -> std::io::Result<()> {
    println!("🦀 Rust is calculating animation data...");

    // 1. Start building the Python script content
    // We add standard Blender boilerplate here.
    let mut script = String::from(r#"
import bpy
import math

# --- Setup Scene ---
# Clear existing mesh objects
bpy.ops.object.select_all(action='DESELECT')
bpy.ops.object.select_by_type(type='MESH')
bpy.ops.object.delete()

# Set end frame
bpy.context.scene.frame_end = "#);
    
    script.push_str(&format!("{}\n", FRAMES));

    // 2. Rust Logic: Calculate positions and write generation code
    // We are generating a line of cubes
    for i in 0..NUM_CUBES {
        let x_pos = i as f32 * 2.5;
        
        // Add code to create a cube at the starting position
        script.push_str(&format!(
            "bpy.ops.mesh.primitive_cube_add(size=2, location=({}, 0, 0))\n", 
            x_pos
        ));
        script.push_str("cube = bpy.context.active_object\n");

        // 3. Rust Logic: Calculate Animation Keyframes
        // We create a wave effect using sin()
        for frame in 0..=FRAMES {
            // The math happens here in RUST, not Python
            // z = sin(frame_time + offset)
            let time_step = frame as f32 * 0.2;
            let offset = i as f32 * 0.5;
            let z_pos = (time_step + offset).sin() * 3.0;

            // Add code to set location and insert keyframe
            script.push_str(&format!(
                "cube.location.z = {:.4}\n", z_pos
            ));
            script.push_str(&format!(
                "cube.keyframe_insert(data_path='location', frame={})\n", frame
            ));
        }
    }

    // 4. Setup Camera and Render Settings via Python
    script.push_str(r#"
# --- Setup Camera ---
camera_data = bpy.data.cameras.new(name='Camera')
camera_object = bpy.data.objects.new('Camera', camera_data)
bpy.context.collection.objects.link(camera_object)
bpy.context.scene.camera = camera_object

# Position camera to look at the cubes
camera_object.location = (12, -25, 10)
camera_object.rotation_euler = (1.1, 0, 0)

# --- Render Settings ---
bpy.context.scene.render.engine = 'BLENDER_EEVEE'
bpy.context.scene.render.image_settings.file_format = 'FFMPEG'
bpy.context.scene.render.ffmpeg.format = 'MPEG4'
bpy.context.scene.render.ffmpeg.codec = 'H264'
bpy.context.scene.render.filepath = '"#);

    script.push_str(RENDER_OUTPUT);
    script.push_str("'\n");

    // 5. Write the script to a file
    let mut file = File::create(OUTPUT_FILENAME)?;
    file.write_all(script.as_bytes())?;
    
    println!("✅ Python script generated successfully.");
    println!("🎥 Launching Blender to render video...");

    // 6. Execute Blender via CLI
    // Try to find Blender in common locations
    let blender_paths = vec![
        "blender", // System PATH
        "/Applications/Blender.app/Contents/MacOS/Blender", // macOS
        "/usr/bin/blender", // Linux
        "C:\\Program Files\\Blender Foundation\\Blender 3.6\\blender.exe", // Windows
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
            // Command: blender -b -P generated_script.py -a
            let output = Command::new(blender)
                .arg("-b")                      // Run in background (headless)
                .arg("-P")                      // Run a python script
                .arg(OUTPUT_FILENAME)           // The script we just made
                .arg("-a")                      // Render animation
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
            eprintln!("❌ Failed to find Blender. Please install it and add to your PATH.");
            eprintln!("Common locations:");
            eprintln!("  macOS: /Applications/Blender.app/Contents/MacOS/Blender");
            eprintln!("  Linux: /usr/bin/blender");
            eprintln!("  Windows: C:\\Program Files\\Blender Foundation\\Blender 3.x\\blender.exe");
            eprintln!("\n📝 The Python script has been generated at: {}", OUTPUT_FILENAME);
            eprintln!("You can manually run: blender -b -P {} -a", OUTPUT_FILENAME);
        }
    }

    Ok(())
}
