//! A collection of officially maintained [postprocessors][crate::Postprocessor].

use super::{Context, MarkdownEvents, PostprocessorResult};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};
use regex::Regex;
use serde_yaml::Value;
use std::string::String;

/// This postprocessor converts all soft line breaks to hard line breaks. Enabling this mimics
/// Obsidian's _'Strict line breaks'_ setting.
pub fn softbreaks_to_hardbreaks(
    _context: &mut Context,
    events: &mut MarkdownEvents,
) -> PostprocessorResult {
    for event in events.iter_mut() {
        if event == &Event::SoftBreak {
            *event = Event::HardBreak;
        }
    }
    PostprocessorResult::Continue
}

pub fn filter_by_tags(
    skip_tags: Vec<String>,
    only_tags: Vec<String>,
) -> impl Fn(&mut Context, &mut MarkdownEvents) -> PostprocessorResult {
    move |context: &mut Context, _events: &mut MarkdownEvents| -> PostprocessorResult {
        match context.frontmatter.get("tags") {
            None => filter_by_tags_(&[], &skip_tags, &only_tags),
            Some(Value::Sequence(tags)) => filter_by_tags_(tags, &skip_tags, &only_tags),
            _ => PostprocessorResult::Continue,
        }
    }
}

fn filter_by_tags_(
    tags: &[Value],
    skip_tags: &[String],
    only_tags: &[String],
) -> PostprocessorResult {
    let skip = skip_tags
        .iter()
        .any(|tag| tags.contains(&Value::String(tag.to_string())));
    let include = only_tags.is_empty()
        || only_tags
            .iter()
            .any(|tag| tags.contains(&Value::String(tag.to_string())));

    if skip || !include {
        PostprocessorResult::StopAndSkipNote
    } else {
        PostprocessorResult::Continue
    }
}

pub fn remove_toc(_context: &mut Context, events: &mut MarkdownEvents) -> PostprocessorResult {
    let mut output = Vec::with_capacity(events.len());

    for event in &mut *events {
        output.push(event.to_owned());
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref language_tag))) => {
                if language_tag != &CowStr::from("toc")
                    && language_tag != &CowStr::from("table-of-contents")
                {
                    continue;
                }
                output.pop(); // Remove codeblock start tag that was pushed onto output
            }
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(ref language_tag))) => {
                if language_tag == &CowStr::from("toc")
                    && language_tag != &CowStr::from("table-of-contents")
                {
                    // The corresponding codeblock start tag for this is replaced with regular
                    // text (containing the Hugo shortcode), so we must also pop this end tag.
                    output.pop();
                }
            }
            _ => {}
        }
    }
    *events = output;
    PostprocessorResult::Continue
}

pub fn remove_obsidian_comments(
    _context: &mut Context,
    events: &mut MarkdownEvents,
) -> PostprocessorResult {
    let mut output = Vec::with_capacity(events.len());
    let mut inside_comment = false;
    let mut inside_codeblock = false;

    for event in &mut *events {
        output.push(event.to_owned());

        match event {
            Event::Text(ref text) => {
                if !text.contains("%%") {
                    if inside_comment {
                        output.pop();
                    }
                    continue;
                } else if inside_codeblock {
                    continue;
                }

                output.pop();

                if inside_comment {
                    inside_comment = false;
                    continue;
                }

                if !text.eq(&CowStr::from("%%")) {
                    let re = Regex::new(r"%%.*?%%").unwrap();
                    let result = re.replace_all(text, "").to_string();
                    output.push(Event::Text(CowStr::from(result)));
                    continue;
                }

                inside_comment = true;
            }
            Event::Start(Tag::CodeBlock(_)) => {
                inside_codeblock = true;
            }
            Event::End(Tag::CodeBlock(_)) => {
                inside_codeblock = false;
            }

            _ => {
                if inside_comment {
                    output.pop();
                }
            }
        }
    }

    *events = output;
    PostprocessorResult::Continue
}

#[test]
fn test_filter_tags() {
    let tags = vec![
        Value::String("skip".to_string()),
        Value::String("publish".to_string()),
    ];
    let empty_tags = vec![];
    assert_eq!(
        filter_by_tags_(&empty_tags, &[], &[]),
        PostprocessorResult::Continue,
        "When no exclusion & inclusion are specified, files without tags are included"
    );
    assert_eq!(
        filter_by_tags_(&tags, &[], &[]),
        PostprocessorResult::Continue,
        "When no exclusion & inclusion are specified, files with tags are included"
    );
    assert_eq!(
        filter_by_tags_(&tags, &["exclude".to_string()], &[]),
        PostprocessorResult::Continue,
        "When exclusion tags don't match files with tags are included"
    );
    assert_eq!(
        filter_by_tags_(&empty_tags, &["exclude".to_string()], &[]),
        PostprocessorResult::Continue,
        "When exclusion tags don't match files without tags are included"
    );
    assert_eq!(
        filter_by_tags_(&tags, &[], &["publish".to_string()]),
        PostprocessorResult::Continue,
        "When exclusion tags don't match files with tags are included"
    );
    assert_eq!(
        filter_by_tags_(&empty_tags, &[], &["include".to_string()]),
        PostprocessorResult::StopAndSkipNote,
        "When inclusion tags are specified files without tags are excluded"
    );
    assert_eq!(
        filter_by_tags_(&tags, &[], &["include".to_string()]),
        PostprocessorResult::StopAndSkipNote,
        "When exclusion tags don't match files with tags are exluded"
    );
    assert_eq!(
        filter_by_tags_(&tags, &["skip".to_string()], &["skip".to_string()]),
        PostprocessorResult::StopAndSkipNote,
        "When both inclusion and exclusion tags are the same exclusion wins"
    );
    assert_eq!(
        filter_by_tags_(&tags, &["skip".to_string()], &["publish".to_string()]),
        PostprocessorResult::StopAndSkipNote,
        "When both inclusion and exclusion tags match exclusion wins"
    );
}
