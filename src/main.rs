//  Copyright (C) 2023 IBM Corp.
//
//  This library is free software; you can redistribute it and/or
//  modify it under the terms of the GNU Lesser General Public
//  License as published by the Free Software Foundation; either
//  version 2.1 of the License, or (at your option) any later version.
//
//  This library is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
//  Lesser General Public License for more details.
//
//  You should have received a copy of the GNU Lesser General Public
//  License along with this library; if not, write to the Free Software
//  Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301
//  USA

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing_subscriber;

use bigiron_virt::api;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create { model_file: PathBuf },
    List,
    Destroy { id: String },
}

fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match &args.command {
        Commands::Create { model_file } => {
            create_resources_from_file(model_file);
        }
        Commands::List => list_machines(),
        Commands::Destroy { id } => destroy_machine(id),
    }
}

fn create_resources_from_file(model_file: &std::path::Path) {
    let data = std::fs::read_to_string(&model_file).unwrap();
    api::create_from_yaml(&data).unwrap();
}

fn list_machines() {
    println!("{}\t{}", "ID", "STATUS");
    for stat in api::list_machines().expect("error listing machines") {
        println!("{}\t{}", stat.id, stat.status);
    }
}

fn destroy_machine(id: &str) {
    match api::destroy_machine(id) {
        Err(e) => println!("{}", e),
        Ok(_) => println!("Destroyed {}", id),
    }
}
