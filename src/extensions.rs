//! Extension traits and helpers tailored for bytemate bot patterns.
//!
//! Provides:
//! - [`DataAccess`] trait for typed access to shared bot data (database, localization)
//! - [`GuildLanguageProvider`] / [`Translator`] traits for localization
//! - [`ComponentsV2Reply`] helpers for Discord Components V2 text containers
//! - [`registry`] module for command-to-module mapping via the `module` field

use crate::serenity_prelude as serenity;
use std::future::Future;

/// Trait for typed access to shared services on your Data struct.
///
/// # Example
/// ```ignore
/// impl DataAccess for Data {
///     type Database = byteorm_client::Client;
///     type Locales = LocalizationManager;
///     fn database(&self) -> &Arc<Self::Database> { &self.db }
///     fn locales(&self) -> &Arc<Self::Locales> { &self.localization_manager }
/// }
/// ```
pub trait DataAccess: Send + Sync {
    /// Database client type
    type Database: Send + Sync;
    /// Localization manager type
    type Locales: Send + Sync;

    /// Returns a reference to the database client
    fn database(&self) -> &std::sync::Arc<Self::Database>;

    /// Returns a reference to the localization manager
    fn locales(&self) -> &std::sync::Arc<Self::Locales>;
}

/// Trait for resolving a guild's language.
///
/// Implement this on your localization manager to enable automatic language detection.
pub trait GuildLanguageProvider: Send + Sync {
    /// The language type returned (must have a sensible Default for DMs / fallback)
    type Language: Default + Clone + Send;

    /// Get the language for a guild, falling back to default if not found
    fn get_guild_language(
        &self,
        guild_id: serenity::GuildId,
    ) -> impl Future<Output = Self::Language> + Send;
}

/// Trait for translating localization keys to strings.
pub trait Translator: Send + Sync {
    /// The language type used for lookups
    type Language: Clone + Send;

    /// Translate a key for the given language
    fn translate(&self, key: &str, lang: &Self::Language) -> String;

    /// Translate a key with named arguments for the given language
    fn translate_with_args(
        &self,
        key: &str,
        lang: &Self::Language,
        args: &[(&str, String)],
    ) -> String;
}

/// Helpers for building Discord Components V2 replies.
///
/// The bot's primary reply pattern uses `MessageFlags::IS_COMPONENTS_V2` with
/// `Container > TextDisplay` components and an accent color. These helpers
/// reduce the boilerplate from ~8 lines to 1.
pub mod components_v2 {
    use crate::serenity_prelude as serenity;
    use serenity::{
        Colour, CreateAllowedMentions, CreateComponent, CreateContainer, CreateTextDisplay,
        MessageFlags,
    };

    /// Creates a single `Container(TextDisplay(text))` component with the given accent color.
    ///
    /// This is the most common UI pattern in the bot:
    /// ```ignore
    /// let comp = text_container("Hello!", color);
    /// // Equivalent to:
    /// // CreateComponent::Container(
    /// //     CreateContainer::new(vec![CreateComponent::TextDisplay(
    /// //         CreateTextDisplay::new("Hello!"),
    /// //     )]).accent_color(color),
    /// // )
    /// ```
    pub fn text_container<'a>(
        text: impl Into<std::borrow::Cow<'a, str>>,
        color: Colour,
    ) -> CreateComponent<'a> {
        CreateComponent::Container(
            CreateContainer::new(vec![CreateComponent::TextDisplay(
                CreateTextDisplay::new(text),
            )])
            .accent_color(color),
        )
    }

    /// Creates a `Container` component with the given inner components and accent color.
    pub fn container<'a>(
        components: Vec<CreateComponent<'a>>,
        color: Colour,
    ) -> CreateComponent<'a> {
        CreateComponent::Container(
            CreateContainer::new(components).accent_color(color),
        )
    }

    /// Creates a `CreateReply` with Components V2 flags, reply mode, and no mentions.
    ///
    /// This is the standard reply wrapper used in most economy/moderation commands:
    /// ```ignore
    /// let reply = components_v2_reply(vec![text_container("Done!", color)]);
    /// ctx.send(reply).await?;
    /// ```
    pub fn components_v2_reply<'a>(
        components: Vec<CreateComponent<'a>>,
    ) -> crate::CreateReply<'a> {
        crate::CreateReply::default()
            .reply(true)
            .allowed_mentions(CreateAllowedMentions::default())
            .flags(MessageFlags::IS_COMPONENTS_V2)
            .components(components)
    }

    /// Shorthand: single text container reply.
    ///
    /// ```ignore
    /// ctx.send(text_reply("Success!", color)).await?;
    /// ```
    pub fn text_reply<'a>(
        text: impl Into<std::borrow::Cow<'a, str>>,
        color: Colour,
    ) -> crate::CreateReply<'a> {
        components_v2_reply(vec![text_container(text, color)])
    }

    /// Shorthand: single text container reply that is ephemeral.
    pub fn text_reply_ephemeral<'a>(
        text: impl Into<std::borrow::Cow<'a, str>>,
        color: Colour,
    ) -> crate::CreateReply<'a> {
        text_reply(text, color).ephemeral(true)
    }
}

/// Utility functions for command registration and module management.
///
/// Uses the `module` field on [`crate::Command`] (set via `#[poise::command(module = "...")]`)
/// to automatically build command-to-module mappings, replacing manual per-module registration.
pub mod registry {
    /// Builds a mapping of module name → list of command names.
    ///
    /// Commands without a `module` field fall back to `category`, then "Uncategorized".
    pub fn build_command_registry<U, E>(
        commands: &[crate::Command<U, E>],
    ) -> std::collections::HashMap<String, Vec<String>> {
        let mut registry = std::collections::HashMap::new();

        fn collect<U, E>(
            cmd: &crate::Command<U, E>,
            registry: &mut std::collections::HashMap<String, Vec<String>>,
        ) {
            let module = cmd
                .module
                .as_deref()
                .or(cmd.category.as_deref())
                .unwrap_or("Uncategorized");

            registry
                .entry(module.to_string())
                .or_insert_with(Vec::new)
                .push(cmd.name.to_string());

            for subcmd in &cmd.subcommands {
                collect(subcmd, registry);
            }
        }

        for cmd in commands {
            collect(cmd, &mut registry);
        }

        registry
    }

    /// Filter commands by module name.
    pub fn filter_by_module<'a, U, E>(
        commands: &'a [crate::Command<U, E>],
        module: &str,
    ) -> Vec<&'a crate::Command<U, E>> {
        commands
            .iter()
            .filter(|cmd| {
                cmd.module.as_deref() == Some(module) || cmd.category.as_deref() == Some(module)
            })
            .collect()
    }
}
