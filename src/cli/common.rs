//! Shared clap helper types for CLI commands.

use clap::ValueEnum;
use punchcard::{CardImageStyle, CardType, EncodingKind, PageLayout, RenderStyle};

/// Supported encoding flags accepted by CLI commands.
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum EncodingArg {
    Hollerith,
    Ascii,
    Ebcdic,
}

impl From<EncodingArg> for EncodingKind {
    fn from(value: EncodingArg) -> EncodingKind {
        match value {
            EncodingArg::Hollerith => EncodingKind::Hollerith,
            EncodingArg::Ascii => EncodingKind::Ascii,
            EncodingArg::Ebcdic => EncodingKind::Ebcdic,
        }
    }
}

/// Card type selector used by several commands.
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum CardTypeArg {
    Code,
    Data,
    Jcl,
    Comment,
    Separator,
    Patch,
}

impl From<CardTypeArg> for CardType {
    fn from(value: CardTypeArg) -> CardType {
        match value {
            CardTypeArg::Code => CardType::Code,
            CardTypeArg::Data => CardType::Data,
            CardTypeArg::Jcl => CardType::Jcl,
            CardTypeArg::Comment => CardType::Comment,
            CardTypeArg::Separator => CardType::Separator,
            CardTypeArg::Patch => CardType::Patch,
        }
    }
}

/// Render styles available for ASCII punch views.
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum RenderStyleArg {
    #[value(name = "ascii-x")]
    AsciiX,
    #[value(name = "ascii-01")]
    Ascii01,
}

impl From<RenderStyleArg> for RenderStyle {
    fn from(value: RenderStyleArg) -> Self {
        match value {
            RenderStyleArg::AsciiX => RenderStyle::AsciiX,
            RenderStyleArg::Ascii01 => RenderStyle::Ascii01,
        }
    }
}

/// Styles available for PNG rendering.
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum CardImageStyleArg {
    Plain,
    Interpreter,
    Keypunch,
}

impl From<CardImageStyleArg> for CardImageStyle {
    fn from(value: CardImageStyleArg) -> CardImageStyle {
        match value {
            CardImageStyleArg::Plain => CardImageStyle::Plain,
            CardImageStyleArg::Interpreter => CardImageStyle::Interpreter,
            CardImageStyleArg::Keypunch => CardImageStyle::Keypunch,
        }
    }
}

/// Output page layout options for image rendering.
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum PageLayoutArg {
    Card,
    A4,
}

impl From<PageLayoutArg> for PageLayout {
    fn from(value: PageLayoutArg) -> PageLayout {
        match value {
            PageLayoutArg::Card => PageLayout::Card,
            PageLayoutArg::A4 => PageLayout::A4,
        }
    }
}
