use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use crux_core::type_generation::facet::{BoltFfi, Config, TypeRegistry};
use log::info;

use shared::GoVoices;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Language {
    Swift,
    Kotlin,
    Csharp,
    Typescript,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_enum)]
    language: Language,
    #[arg(short, long)]
    output_dir: PathBuf,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    // The Swift and Kotlin shells use the generated `Core` and implement the
    // generated `EffectHandler`. `boltffi pack` puts `CoreFfi` in the `Shared`
    // Swift module and, for Kotlin, in the app's package (`boltffi.toml`).
    let typegen_app = TypeRegistry::new()
        .register_app::<GoVoices>()?
        .build()?
        .boltffi(
            BoltFfi::new()
                .swift("Shared")
                .kotlin_package("com.tszarzynski.govoices"),
        );

    let name = match args.language {
        Language::Swift => "App",
        Language::Kotlin => "com.tszarzynski.govoices",
        Language::Csharp => "GoVoices.Shared",
        Language::Typescript => "app",
    };
    let mut builder = Config::builder(name, &args.output_dir);
    if args.language == Language::Swift {
        // The BoltFFI package the generated one depends on declares these,
        // and SPM won't link a package with a lower deployment target.
        builder.platform(".iOS(.v16)").platform(".macOS(.v13)");
    }
    let config = builder.build();

    match args.language {
        Language::Swift => {
            info!("Typegen for Swift");
            typegen_app.swift(&config)?;
        }
        Language::Kotlin => {
            info!("Typegen for Kotlin");
            typegen_app.kotlin(&config)?;
        }
        Language::Csharp => {
            info!("Typegen for C#");
            typegen_app.csharp(&config)?;
        }
        Language::Typescript => {
            info!("Typegen for TypeScript");
            typegen_app.typescript(&config)?;
        }
    }

    Ok(())
}
