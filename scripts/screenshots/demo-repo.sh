#!/usr/bin/env bash
# Builds the repository the README screenshots are taken in: a small project with a main line on a local
# "origin", a feature branch of three commits, and uncommitted work. Names and dates are made up, and fixed so
# that every run draws the same screens.
set -euo pipefail

root="$1"
origin="$root/origin.git"
work="$root/weather"

author() {
    export GIT_AUTHOR_NAME="$1" GIT_AUTHOR_EMAIL="$2" GIT_COMMITTER_NAME="$1" GIT_COMMITTER_EMAIL="$2"
}

commit() {
    export GIT_AUTHOR_DATE="$1" GIT_COMMITTER_DATE="$1"
    git add -A
    git -c commit.gpgsign=false commit -q -m "$2"
}

git init -q --bare -b main "$origin"
git init -q -b main "$work"
cd "$work"
git remote add origin "$origin"
mkdir -p src tests

author "Ada Lovelace" "ada@example.com"
cat > Cargo.toml <<'EOF'
[package]
name = "weather"
version = "0.3.0"
edition = "2024"
EOF
cat > src/lib.rs <<'EOF'
pub mod forecast;
pub mod parser;
EOF
cat > src/parser.rs <<'EOF'
use crate::forecast::Reading;

/// Reads "city;temperature;humidity" lines, skipping the ones that do not parse.
pub fn parse(input: &str) -> Vec<Reading> {
    input.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<Reading> {
    let mut fields = line.split(';');
    let city = fields.next()?.trim().to_string();
    let temperature = fields.next()?.trim().parse().ok()?;
    let humidity = fields.next()?.trim().parse().ok()?;

    Some(Reading { city, temperature, humidity })
}
EOF
cat > src/forecast.rs <<'EOF'
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    pub city: String,
    pub temperature: f32,
    pub humidity: u8,
}

pub fn average(readings: &[Reading]) -> Option<f32> {
    if readings.is_empty() {
        return None;
    }
    Some(readings.iter().map(|reading| reading.temperature).sum::<f32>() / readings.len() as f32)
}
EOF
commit "2026-10-01T10:12:00+0200" "feat: parse city readings"
git push -q origin main

git switch -q -c feature/search
author "Grace Hopper" "grace@example.com"
cat > src/search.rs <<'EOF'
use crate::forecast::Reading;

/// The readings of a city, whatever the case it was typed in.
pub fn by_city<'a>(readings: &'a [Reading], city: &str) -> Vec<&'a Reading> {
    readings.iter().filter(|reading| reading.city.eq_ignore_ascii_case(city)).collect()
}
EOF
cat > src/lib.rs <<'EOF'
pub mod forecast;
pub mod parser;
pub mod search;
EOF
commit "2026-10-14T16:40:00+0200" "feat(search): find readings by city"

cat > tests/parser_test.rs <<'EOF'
use weather::parser::parse;

#[test]
fn parse_skips_lines_that_do_not_parse() {
    let readings = parse("Paris;21.5;40\nnot a reading\nLyon;19;55");

    assert_eq!(readings.len(), 2);
}
EOF
commit "2026-10-15T11:05:00+0200" "test(parser): cover malformed lines"

author "Ada Lovelace" "ada@example.com"
sed -i.bak 's/    let humidity = fields.next()?.trim().parse().ok()?;/    let humidity = fields.next()?.trim().parse().ok().filter(|value| *value <= 100)?;/' src/parser.rs
rm src/parser.rs.bak
commit "2026-10-16T18:22:00+0200" "fix(parser): reject humidity above 100%"

# Uncommitted work: what the files tab opens on.
sed -i.bak 's/    let temperature = fields.next()?.trim().parse().ok()?;/    let temperature = fields.next()?.trim().replace(",", ".").parse().ok()?;/' src/parser.rs
rm src/parser.rs.bak
cat >> src/search.rs <<'EOF'

/// The warmest reading, if any.
pub fn warmest(readings: &[Reading]) -> Option<&Reading> {
    readings.iter().max_by(|left, right| left.temperature.total_cmp(&right.temperature))
}
EOF
cat >> tests/parser_test.rs <<'EOF'

#[test]
fn parse_reads_a_decimal_comma() {
    assert_eq!(parse("Nice;23,5;60")[0].temperature, 23.5);
}
EOF
cat > src/alerts.rs <<'EOF'
use crate::forecast::Reading;

pub fn heat_warnings(readings: &[Reading]) -> Vec<String> {
    readings.iter().filter(|reading| reading.temperature >= 35.0).map(|reading| format!("Heat warning: {}", reading.city)).collect()
}
EOF
