//! Setup command for installing hooks and configuring task2habitica

use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::Command,
};

use dialoguer::{Confirm, Input, Password};

use crate::error::{Error, Result};

// Embed hook scripts directly into the binary
const HOOK_ON_ADD: &str = include_str!("../../hooks/on-add.task2habitica");
const HOOK_ON_MODIFY: &str = include_str!("../../hooks/on-modify.task2habitica");
const HOOK_ON_EXIT: &str = include_str!("../../hooks/on-exit.task2habitica");

/// UDA definitions to add to .taskrc
const UDA_DEFINITIONS: &str = r"
# task2habitica UDAs
uda.habitica_uuid.label=Habitica UUID
uda.habitica_uuid.type=string

uda.habitica_difficulty.label=Habitica Difficulty
uda.habitica_difficulty.type=string
uda.habitica_difficulty.values=trivial,easy,medium,hard

uda.habitica_task_type.label=Habitica Task Type
uda.habitica_task_type.type=string
uda.habitica_task_type.values=daily,todo
";

/// Hook file information
struct HookFile {
    name: &'static str,
    content: &'static str,
}

const HOOKS: [HookFile; 3] = [
    HookFile {
        name: "on-add.task2habitica",
        content: HOOK_ON_ADD,
    },
    HookFile {
        name: "on-modify.task2habitica",
        content: HOOK_ON_MODIFY,
    },
    HookFile {
        name: "on-exit.task2habitica",
        content: HOOK_ON_EXIT,
    },
];

/// Run the setup command
pub fn handle_setup() -> Result<()> {
    println!();
    println!("task2habitica Setup");
    println!("====================");
    println!();

    // Step 1: Check Taskwarrior
    check_taskwarrior()?;

    // Step 2: Install hooks
    install_hooks()?;

    // Step 3: Configure UDAs
    configure_udas()?;

    // Step 4: Prompt for and validate credentials
    let (user_id, api_key) = prompt_and_validate_credentials()?;

    // Step 5: Write credentials to shell profile
    write_credentials_to_shell(&user_id, &api_key)?;

    println!();
    println!("Setup complete! Your tasks will now sync with Habitica.");
    println!();

    Ok(())
}

/// Check if Taskwarrior is installed and get version
fn check_taskwarrior() -> Result<()> {
    print!("Checking Taskwarrior... ");

    let output = Command::new("task")
        .arg("--version")
        .output()
        .map_err(|_| Error::TaskwarriorNotFound)?;

    if !output.status.success() {
        println!("not found");
        return Err(Error::TaskwarriorNotFound);
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

    println!("found version {}", version);
    Ok(())
}

/// Read a Taskwarrior config value via `task _get`, returning an empty string
/// if it is unset or the command fails.
fn get_taskrc_value(key: &str) -> String {
    Command::new("task")
        .args(["rc.hooks=off", "_get", key])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Get the Taskwarrior hooks directory.
///
/// Resolution order (works for both Taskwarrior 2.x and 3.x):
/// 1. `rc.hooks.location` — explicit override, takes precedence when set.
/// 2. `<rc.data.location>/hooks` — the standard location. `rc.data.location`
///    resolves to `~/.task` on a default 2.x setup and to the configured (often
///    XDG `~/.local/share/task`) directory on 3.x.
/// 3. `~/.task/hooks` — last-resort fallback if Taskwarrior reports nothing.
fn get_hooks_dir() -> Result<PathBuf> {
    // 1. Explicit hooks location override.
    let hooks_location = get_taskrc_value("rc.hooks.location");
    if !hooks_location.is_empty() {
        return expand_path(&hooks_location);
    }

    // 2. Derive from the data location.
    let data_location = get_taskrc_value("rc.data.location");
    if !data_location.is_empty() {
        let path = expand_path(&data_location)?;
        return Ok(path.join("hooks"));
    }

    // 3. Fallback to ~/.task/hooks.
    let home =
        dirs::home_dir().ok_or_else(|| Error::config("Could not determine home directory"))?;
    Ok(home.join(".task").join("hooks"))
}

/// Get the .taskrc path
fn get_taskrc_path() -> Result<PathBuf> {
    // Check TASKRC environment variable first
    if let Ok(taskrc) = env::var("TASKRC") {
        return Ok(PathBuf::from(taskrc));
    }

    // Default to ~/.taskrc
    let home =
        dirs::home_dir().ok_or_else(|| Error::config("Could not determine home directory"))?;
    Ok(home.join(".taskrc"))
}

/// Expand ~ in paths
fn expand_path(path: &str) -> Result<PathBuf> {
    if let Some(stripped) = path.strip_prefix('~') {
        let home =
            dirs::home_dir().ok_or_else(|| Error::config("Could not determine home directory"))?;
        let rest = stripped.strip_prefix('/').unwrap_or(stripped);
        Ok(home.join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

/// Install hook scripts
fn install_hooks() -> Result<()> {
    println!();
    println!("Installing hooks...");

    let hooks_dir = get_hooks_dir()?;

    // Create hooks directory if it doesn't exist
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir)
            .map_err(|e| Error::config(format!("Failed to create hooks directory: {}", e)))?;
        println!("  Created {}", hooks_dir.display());
    }

    for hook in &HOOKS {
        let hook_path = hooks_dir.join(hook.name);

        // Check if hook already exists
        if hook_path.exists() {
            let overwrite = Confirm::new()
                .with_prompt(format!("  Hook '{}' already exists. Overwrite?", hook.name))
                .default(false)
                .interact()
                .map_err(|e| Error::config(format!("Failed to read input: {}", e)))?;

            if !overwrite {
                println!("  Skipped {}", hook.name);
                continue;
            }
        }

        // Write hook file
        fs::write(&hook_path, hook.content)
            .map_err(|e| Error::config(format!("Failed to write hook {}: {}", hook.name, e)))?;

        // Make executable (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&hook_path)
                .map_err(|e| Error::config(format!("Failed to read hook permissions: {}", e)))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms)
                .map_err(|e| Error::config(format!("Failed to set hook permissions: {}", e)))?;
        }

        println!("  Installed {}", hook.name);
    }

    Ok(())
}

/// Configure UDAs in .taskrc
fn configure_udas() -> Result<()> {
    println!();
    println!("Configuring Taskwarrior UDAs...");

    let taskrc_path = get_taskrc_path()?;

    // Read existing .taskrc content
    let existing_content = if taskrc_path.exists() {
        fs::read_to_string(&taskrc_path)
            .map_err(|e| Error::config(format!("Failed to read .taskrc: {}", e)))?
    } else {
        String::new()
    };

    // Check which UDAs are already present
    let udas_to_add = [
        ("uda.habitica_uuid", "habitica_uuid"),
        ("uda.habitica_difficulty", "habitica_difficulty"),
        ("uda.habitica_task_type", "habitica_task_type"),
    ];

    let mut any_added = false;
    let mut missing_udas = Vec::new();

    for (uda_key, uda_name) in &udas_to_add {
        if existing_content.contains(uda_key) {
            println!("  {} (already present)", uda_name);
        } else {
            missing_udas.push(*uda_name);
        }
    }

    // Add missing UDAs
    if !missing_udas.is_empty() {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&taskrc_path)
            .map_err(|e| Error::config(format!("Failed to open .taskrc: {}", e)))?;

        // Add a newline before UDAs if file doesn't end with one
        if !existing_content.is_empty() && !existing_content.ends_with('\n') {
            writeln!(file)
                .map_err(|e| Error::config(format!("Failed to write to .taskrc: {}", e)))?;
        }

        // Write UDA definitions (only write if we're adding new ones)
        if !existing_content.contains("# task2habitica UDAs") {
            write!(file, "{}", UDA_DEFINITIONS)
                .map_err(|e| Error::config(format!("Failed to write UDAs to .taskrc: {}", e)))?;
        }

        for uda_name in &missing_udas {
            println!("  {} (added)", uda_name);
        }
        any_added = true;
    }

    if !any_added && missing_udas.is_empty() {
        println!("  All UDAs already configured");
    }

    Ok(())
}

/// Check if credentials are already configured
fn check_existing_credentials() -> (Option<String>, Option<String>) {
    // Check environment variables
    let user_id = env::var("HABITICA_USER_ID").ok().filter(|s| !s.is_empty());
    let api_key = env::var("HABITICA_API_KEY").ok().filter(|s| !s.is_empty());

    if user_id.is_some() && api_key.is_some() {
        return (user_id, api_key);
    }

    // Check .taskrc
    if let Ok(output) = Command::new("task")
        .args(["rc.hooks=off", "_get", "rc.habitica.user_id"])
        .output()
    {
        let taskrc_user_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !taskrc_user_id.is_empty() && user_id.is_none() {
            return (Some(taskrc_user_id), api_key);
        }
    }

    (user_id, api_key)
}

/// Prompt for credentials and validate them
fn prompt_and_validate_credentials() -> Result<(String, String)> {
    println!();
    println!("----------------------------------------");
    println!();
    println!("Habitica Credentials");
    println!("Find your User ID and API Key at:");
    println!("  https://habitica.com/user/settings/api");
    println!();

    // Check for existing credentials
    let (existing_user_id, existing_api_key) = check_existing_credentials();

    if let (Some(ref user_id), Some(ref api_key)) = (&existing_user_id, &existing_api_key) {
        println!("Credentials already configured in environment.");
        let reconfigure = Confirm::new()
            .with_prompt("Reconfigure credentials?")
            .default(false)
            .interact()
            .map_err(|e| Error::config(format!("Failed to read input: {}", e)))?;

        if !reconfigure {
            // Validate existing credentials
            print!("Validating existing credentials... ");
            match validate_credentials(user_id, api_key) {
                Ok(()) => {
                    println!("valid");
                    return Ok((user_id.clone(), api_key.clone()));
                }
                Err(e) => {
                    println!("failed");
                    println!("  Error: {}", e);
                    println!();
                    println!("Please enter new credentials:");
                }
            }
        }
    }

    // Prompt for User ID
    let user_id: String = Input::new()
        .with_prompt("User ID")
        .interact_text()
        .map_err(|e| Error::config(format!("Failed to read User ID: {}", e)))?;

    // Prompt for API Key (masked input)
    let api_key: String = Password::new()
        .with_prompt("API Key")
        .interact()
        .map_err(|e| Error::config(format!("Failed to read API Key: {}", e)))?;

    // Validate credentials
    print!("Validating credentials... ");
    validate_credentials(&user_id, &api_key)?;
    println!("success!");

    Ok((user_id, api_key))
}

/// Validate Habitica credentials by making an API call
fn validate_credentials(user_id: &str, api_key: &str) -> Result<()> {
    let client = reqwest::blocking::Client::new();

    let response = client
        .get("https://habitica.com/api/v4/user")
        .header("x-api-user", user_id)
        .header("x-api-key", api_key)
        .header(
            "x-client",
            "cab16cfa-e951-4dc3-a468-1abadc1dd109-Task2HabiticaRust",
        )
        .send()
        .map_err(|e| Error::HabiticaApiError(format!("Connection failed: {}", e)))?;

    if response.status().is_success() {
        Ok(())
    } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        Err(Error::InvalidHabiticaCredentials)
    } else {
        Err(Error::HabiticaApiError(format!(
            "HTTP {}: {}",
            response.status(),
            response.text().unwrap_or_default()
        )))
    }
}

/// Detect the user's shell type
fn detect_shell() -> (&'static str, PathBuf) {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    // Check SHELL environment variable
    if let Ok(shell) = env::var("SHELL") {
        if shell.contains("zsh") {
            return ("zsh", home.join(".zshrc"));
        } else if shell.contains("fish") {
            return ("fish", home.join(".config/fish/config.fish"));
        } else if shell.contains("bash") {
            // Check for .bash_profile on macOS, .bashrc on Linux
            let bash_profile = home.join(".bash_profile");
            if bash_profile.exists() {
                return ("bash", bash_profile);
            }
            return ("bash", home.join(".bashrc"));
        }
    }

    // Default to bash
    ("bash", home.join(".bashrc"))
}

/// Write credentials to shell profile
fn write_credentials_to_shell(user_id: &str, api_key: &str) -> Result<()> {
    println!();
    println!("----------------------------------------");
    println!();

    let (shell_name, profile_path) = detect_shell();
    println!("Detected shell: {}", shell_name);

    let export_lines = if shell_name == "fish" {
        format!(
            r#"
# task2habitica Habitica credentials
set -gx HABITICA_USER_ID "{}"
set -gx HABITICA_API_KEY "{}"
"#,
            user_id, api_key
        )
    } else {
        format!(
            r#"
# task2habitica Habitica credentials
export HABITICA_USER_ID="{}"
export HABITICA_API_KEY="{}"
"#,
            user_id, api_key
        )
    };

    // Check if credentials are already in the profile
    if profile_path.exists() {
        let content = fs::read_to_string(&profile_path).unwrap_or_default();
        if content.contains("HABITICA_USER_ID") && content.contains("HABITICA_API_KEY") {
            println!("Credentials already present in {}", profile_path.display());

            let update = Confirm::new()
                .with_prompt("Update credentials?")
                .default(false)
                .interact()
                .map_err(|e| Error::config(format!("Failed to read input: {}", e)))?;

            if !update {
                println!();
                println!("To apply changes, run:");
                println!("  source {}", profile_path.display());
                return Ok(());
            }

            // Remove old credentials and add new ones
            let mut new_content = String::new();
            let mut skip_until_empty = false;

            for line in content.lines() {
                if line.contains("# task2habitica Habitica credentials") {
                    skip_until_empty = true;
                    continue;
                }
                if skip_until_empty {
                    if line.trim().is_empty() {
                        skip_until_empty = false;
                    }
                    continue;
                }
                if line.contains("HABITICA_USER_ID") || line.contains("HABITICA_API_KEY") {
                    continue;
                }
                new_content.push_str(line);
                new_content.push('\n');
            }

            // Append new credentials
            new_content.push_str(&export_lines);

            fs::write(&profile_path, new_content)
                .map_err(|e| Error::config(format!("Failed to write to profile: {}", e)))?;

            println!("Updated credentials in {}", profile_path.display());
            println!();
            println!("To apply changes, run:");
            println!("  source {}", profile_path.display());
            return Ok(());
        }
    }

    // Ask to add credentials to profile
    let add_to_profile = Confirm::new()
        .with_prompt(format!("Add credentials to {}?", profile_path.display()))
        .default(true)
        .interact()
        .map_err(|e| Error::config(format!("Failed to read input: {}", e)))?;

    if add_to_profile {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&profile_path)
            .map_err(|e| Error::config(format!("Failed to open profile: {}", e)))?;

        write!(file, "{}", export_lines)
            .map_err(|e| Error::config(format!("Failed to write to profile: {}", e)))?;

        println!("Credentials written to {}", profile_path.display());
        println!();
        println!("To apply changes, run:");
        println!("  source {}", profile_path.display());
    } else {
        println!();
        println!("Add these lines to your shell profile:");
        println!("{}", export_lines);
    }

    Ok(())
}
