use std::{
    io::Write,
    path::Path,
    sync::{Arc, mpsc},
};

use octamed::mmd::{conversion::FromModule, parser::OctamedMMDParser};
use octamedic_core::{
    data::{project::OctamedicProject, song::SongId},
    engine::{
        engine::{OctamedicEngine, PlayMode},
        handle::{EngineCommand, EngineStatus},
    },
};

const SAMPLE_RATE: u32 = 48000;

/// Renders a whole example song offline, as the audio callback would.

fn render(file: &str) -> Vec<f32> {

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../octamed_cli/example_meds")
        .join(file);

    let mmd = OctamedMMDParser::new()
        .parse_file(&path)
        .unwrap()
        .remove(0);

    let project = OctamedicProject::from_module(&mmd);

    let (commands, receiver) = mpsc::channel();

    let status = Arc::new(EngineStatus::default());

    let mut engine = OctamedicEngine::new(receiver, status.clone(), SAMPLE_RATE);

    for command in EngineCommand::load_song(project.get_song(SongId(0)).unwrap()) {

        commands.send(command).unwrap();
    }

    commands
        .send(EngineCommand::PlaySong {
            sequence_start: 0,
            looping: false,
        })
        .unwrap();

    let mut rendered = vec![];

    let mut buffer = vec![0.0; 512];

    // ten minutes at most, in case the song never ends.
    for _ in 0..(SAMPLE_RATE as usize * 600 / buffer.len()) {

        engine.process(&mut buffer, 1);

        rendered.extend_from_slice(&buffer);

        if status.play_mode() == PlayMode::Paused {

            break;
        }
    }

    return rendered;
}

/// Writes mono 16-bit PCM to `path`, for listening to a render outside the engine.

fn write_wav(path: &Path, samples: &[f32]) {

    let data_size = samples.len() as u32 * 2;

    let mut bytes = Vec::with_capacity(44 + data_size as usize);

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());

    for sample in samples {

        bytes.extend_from_slice(&((sample * i16::MAX as f32) as i16).to_le_bytes());
    }

    std::fs::File::create(path)
        .unwrap()
        .write_all(&bytes)
        .unwrap();
}

#[test]

fn example_songs_render_and_end() {

    for file in ["example.mmd0", "example.mmd1", "Bellsv2 mono.med"] {

        let rendered = render(file);

        let peak = rendered.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));

        assert!(peak > 0.1, "{file} is silent");

        assert!(peak <= 1.0);

        assert!(rendered.len() < SAMPLE_RATE as usize * 600, "{file} never ended");
    }
}

/// Writes the example songs to wav files when `OCTAMEDIC_RENDER_DIR` is set.

#[test]

fn export_example_renders() {

    let Ok(dir) = std::env::var("OCTAMEDIC_RENDER_DIR") else {

        return;
    };

    for file in ["example.mmd0", "example.mmd1", "Bellsv2 mono.med"] {

        write_wav(&Path::new(&dir).join(format!("{file}.wav")), &render(file));
    }
}
