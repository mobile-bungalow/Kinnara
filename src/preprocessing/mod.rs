mod sampler_hint;
mod uniform_hint;
mod var_hint;

use derive_more::From;
use std::num::NonZeroU32;
use struct_patch::Patch;
use thiserror::Error;
use wgpu::naga::{FastHashMap, ResourceBinding};

pub(crate) use sampler_hint::*;
pub(crate) use uniform_hint::*;

#[derive(Debug, Error)]
pub enum PreprocessingError {
    #[error("Error Parsing : {0}")]
    ParsingError(String),
    #[error("Unsupported source type, only wgsl and glsl supported.")]
    UnsupportedSource,
}

#[derive(Debug)]
pub struct TextureHint {}

#[derive(Debug)]
pub struct ImageHint {}

#[derive(Debug, Default)]
pub struct Directives {
    uniform_hint_base: UniformHintPatch,
    uniform_hints: FastHashMap<ResourceBinding, UniformHint>,

    sampler_hint_base: SamplerHintPatch,
    sampler_hint: FastHashMap<ResourceBinding, SamplerHint>,
}

impl Directives {
    /// Returns uniform hint, patched with defaults if
    /// unreferenced in the directives
    pub fn get_uniform_hint(&self, binding: &ResourceBinding) -> UniformHint {
        let mut hint = self.uniform_hints.get(binding).cloned().unwrap_or_default();
        hint.apply(self.uniform_hint_base.clone());
        hint
    }

    pub fn get_sampler_hint(&self, binding: &ResourceBinding) -> SamplerHint {
        let mut hint = self.sampler_hint.get(binding).cloned().unwrap_or_default();
        hint.apply(self.sampler_hint_base.clone());
        hint
    }
}

pub fn process<'a>(
    source: &'a wgpu::ShaderSource,
) -> Result<(Directives, wgpu::ShaderSource<'a>), PreprocessingError> {
    let deleteme_out = source.to_owned().clone();
    let mut directives = Default::default();

    let src = match source {
        wgpu::ShaderSource::Glsl { shader, .. } => shader,
        wgpu::ShaderSource::Wgsl(src) => src,
        _ => return Err(PreprocessingError::UnsupportedSource),
    };

    Ok((directives, deleteme_out))
}

#[derive(Debug, From)]
enum Directive {
    Texture(TextureHint),
    Buffer(UniformHint),
    Var(var_hint::GlobalVarHint),
    Image(ImageHint),
    Sampler(SamplerHint),
}

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{alpha1, alphanumeric1, char, digit1, multispace0, one_of},
    combinator::{map, map_res, opt, recognize},
    multi::{many0, many1, separated_list0, separated_list1},
    number::complete::float,
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};

fn parse_pragma() {}

fn parse_identifier(input: &str) -> IResult<&str, String> {
    map(
        recognize(pair(
            alt((alpha1, tag("_"))),
            many0(alt((alphanumeric1, tag("_")))),
        )),
        |s: &str| s.to_string(),
    )(input)
}

fn parse_count(input: &str) -> IResult<&str, Option<NonZeroU32>> {
    opt(delimited(
        char('['),
        map_res(digit1, |s: &str| s.parse::<NonZeroU32>()),
        char(']'),
    ))(input)
}

fn parse_global_access_name(input: &str) -> IResult<&str, Vec<String>> {
    separated_list1(char('.'), parse_identifier)(input)
}
