/// Convert LaTeX math fragments inside $...$ or $$...$$ to Unicode approximations.
use std::collections::HashMap;

pub fn latex_to_unicode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    let mut cmd_map = command_map();
    let mut greek_map = greek_map();

    while let Some((_i, ch)) = chars.next() {
        if ch != '$' {
            out.push(ch);
            continue;
        }

        let is_block = chars.peek().map_or(false, |(_, c)| *c == '$');
        if is_block {
            chars.next(); // consume second $
        }

        let _end_marker = if is_block { "$$" } else { "$" };
        let mut math = String::new();
        let mut found_end = false;
        while let Some((_, c)) = chars.next() {
            if c == '$' && (!is_block || chars.peek().map_or(false, |(_, nc)| *nc == '$')) {
                if is_block {
                    chars.next(); // consume second $
                }
                found_end = true;
                break;
            }
            math.push(c);
        }

        if found_end {
            let converted = convert_math(&math, &mut cmd_map, &mut greek_map);
            if is_block {
                out.push('\n');
                out.push_str(&converted);
                out.push('\n');
            } else {
                out.push_str(&converted);
            }
        } else {
            out.push('$');
            if is_block { out.push('$'); }
            out.push_str(&math);
        }
    }

    out
}

fn convert_math(math: &str, cmd_map: &mut HashMap<String, String>, greek_map: &mut HashMap<String, String>) -> String {
    let mut out = String::with_capacity(math.len());
    let mut chars = math.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphabetic() {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if name.is_empty() {
                    out.push('\\');
                } else if let Some(repl) = greek_map.get(&name) {
                    out.push_str(repl);
                } else if let Some(repl) = cmd_map.get(&name) {
                    out.push_str(repl);
                } else {
                    out.push('\\');
                    out.push_str(&name);
                }
            }
            '^' => {
                let sup = grab_braced(&mut chars);
                let mapped = superscript(&sup);
                out.push_str(&mapped);
            }
            '_' => {
                let sub = grab_braced(&mut chars);
                let mapped = subscript(&sub);
                out.push_str(&mapped);
            }
            '{' | '}' => {} // skip braces
            '&' => out.push_str("  "), // align separator → spaces
            _ => out.push(ch),
        }
    }

    out
}

fn grab_braced(chars: &mut std::iter::Peekable<impl Iterator<Item = char>>) -> String {
    if chars.peek().map_or(false, |&c| c == '{') {
        chars.next(); // skip {
        let mut buf = String::new();
        let mut depth = 1;
        for c in chars.by_ref() {
            if c == '{' { depth += 1; }
            if c == '}' { depth -= 1; if depth == 0 { break; } }
            buf.push(c);
        }
        buf
    } else {
        let mut c = chars.next().unwrap_or(' ').to_string();
        // Also grab following digits
        while let Some(&d) = chars.peek() {
            if d.is_ascii_digit() {
                c.push(chars.next().unwrap());
            } else {
                break;
            }
        }
        c
    }
}

fn superscript(s: &str) -> String {
    s.chars().map(|c| super_char(c)).collect()
}

fn subscript(s: &str) -> String {
    s.chars().map(|c| sub_char(c)).collect()
}

fn super_char(c: char) -> char {
    match c {
        '0' => '⁰', '1' => '¹', '2' => '²', '3' => '³', '4' => '⁴',
        '5' => '⁵', '6' => '⁶', '7' => '⁷', '8' => '⁸', '9' => '⁹',
        '+' => '⁺', '-' => '⁻', '=' => '⁼', '(' => '⁽', ')' => '⁾',
        'n' => 'ⁿ', 'i' => 'ⁱ',
        _ => c,
    }
}

fn sub_char(c: char) -> char {
    match c {
        '0' => '₀', '1' => '₁', '2' => '₂', '3' => '₃', '4' => '₄',
        '5' => '₅', '6' => '₆', '7' => '₇', '8' => '₈', '9' => '₉',
        '+' => '₊', '-' => '₋', '=' => '₌', '(' => '₍', ')' => '₎',
        'a' => 'ₐ', 'e' => 'ₑ', 'i' => 'ᵢ', 'j' => 'ⱼ', 'n' => 'ₙ',
        'x' => 'ₓ',
        _ => c,
    }
}

fn command_map() -> HashMap<String, String> {
    HashMap::from([
        ("sum".into(), "∑".into()),
        ("prod".into(), "∏".into()),
        ("int".into(), "∫".into()),
        ("iint".into(), "∬".into()),
        ("iiint".into(), "∭".into()),
        ("oint".into(), "∮".into()),
        ("infty".into(), "∞".into()),
        ("partial".into(), "∂".into()),
        ("nabla".into(), "∇".into()),
        ("sqrt".into(), "√".into()),
        ("times".into(), "×".into()),
        ("cdot".into(), "·".into()),
        ("div".into(), "÷".into()),
        ("pm".into(), "±".into()),
        ("mp".into(), "∓".into()),
        ("leq".into(), "≤".into()),
        ("geq".into(), "≥".into()),
        ("neq".into(), "≠".into()),
        ("approx".into(), "≈".into()),
        ("equiv".into(), "≡".into()),
        ("propto".into(), "∝".into()),
        ("sim".into(), "∼".into()),
        ("simeq".into(), "≃".into()),
        ("ll".into(), "≪".into()),
        ("gg".into(), "≫".into()),
        ("to".into(), "→".into()),
        ("rightarrow".into(), "→".into()),
        ("leftarrow".into(), "←".into()),
        ("leftrightarrow".into(), "↔".into()),
        ("Rightarrow".into(), "⇒".into()),
        ("Leftarrow".into(), "⇐".into()),
        ("Leftrightarrow".into(), "⇔".into()),
        ("mapsto".into(), "↦".into()),
        ("implies".into(), "⇒".into()),
        ("iff".into(), "⇔".into()),
        ("land".into(), "∧".into()),
        ("lor".into(), "∨".into()),
        ("lnot".into(), "¬".into()),
        ("neg".into(), "¬".into()),
        ("forall".into(), "∀".into()),
        ("exists".into(), "∃".into()),
        ("ni".into(), "∋".into()),
        ("in".into(), "∈".into()),
        ("notin".into(), "∉".into()),
        ("subset".into(), "⊂".into()),
        ("supset".into(), "⊃".into()),
        ("subseteq".into(), "⊆".into()),
        ("supseteq".into(), "⊇".into()),
        ("cup".into(), "∪".into()),
        ("cap".into(), "∩".into()),
        ("emptyset".into(), "∅".into()),
        ("varnothing".into(), "∅".into()),
        ("angle".into(), "∠".into()),
        ("triangle".into(), "△".into()),
        ("perp".into(), "⊥".into()),
        ("parallel".into(), "∥".into()),
        ("cdot".into(), "·".into()),
        ("dots".into(), "…".into()),
        ("cdots".into(), "⋯".into()),
        ("vdots".into(), "⋮".into()),
        ("ddots".into(), "⋱".into()),
        ("circ".into(), "∘".into()),
        ("bullet".into(), "•".into()),
        ("square".into(), "□".into()),
        ("Box".into(), "□".into()),
        ("diamond".into(), "⋄".into()),
        ("star".into(), "⋆".into()),
        ("bigcup".into(), "⋃".into()),
        ("bigcap".into(), "⋂".into()),
        ("bigvee".into(), "⋁".into()),
        ("bigwedge".into(), "⋀".into()),
        ("bigoplus".into(), "⊕".into()),
        ("bigotimes".into(), "⊗".into()),
        ("oplus".into(), "⊕".into()),
        ("ominus".into(), "⊖".into()),
        ("otimes".into(), "⊗".into()),
        ("oslash".into(), "⊘".into()),
        ("odot".into(), "⊙".into()),
        ("therefore".into(), "∴".into()),
        ("because".into(), "∵".into()),
        ("mid".into(), "∣".into()),
        ("nmid".into(), "∤".into()),
        ("langle".into(), "⟨".into()),
        ("rangle".into(), "⟩".into()),
        ("lceil".into(), "⌈".into()),
        ("rceil".into(), "⌉".into()),
        ("lfloor".into(), "⌊".into()),
        ("rfloor".into(), "⌋".into()),
        ("left".into(), "".into()),
        ("right".into(), "".into()),
        ("frac".into(), "".into()),  // handled inline: a/b
        ("text".into(), "".into()),
        ("mathrm".into(), "".into()),
        ("mathbf".into(), "".into()),
        ("mathit".into(), "".into()),
        ("mathbb".into(), "".into()),
        ("mathcal".into(), "".into()),
        ("bar".into(), "".into()),
        ("hat".into(), "".into()),
        ("tilde".into(), "~".into()),
        ("vec".into(), "".into()),
        ("dot".into(), "".into()),
        ("ddot".into(), "".into()),
        ("overline".into(), "".into()),
        ("underline".into(), "".into()),
        ("quad".into(), "  ".into()),
        ("qquad".into(), "    ".into()),
        (",".into(), "".into()),
        ("\\;".into(), " ".into()),
        ("\\:".into(), " ".into()),
        ("\\!".into(), "".into()),
        ("\\\\".into(), "\n".into()),
    ])
}

fn greek_map() -> HashMap<String, String> {
    HashMap::from([
        ("alpha".into(), "α".into()),
        ("beta".into(), "β".into()),
        ("gamma".into(), "γ".into()),
        ("delta".into(), "δ".into()),
        ("epsilon".into(), "ε".into()),
        ("varepsilon".into(), "ɛ".into()),
        ("zeta".into(), "ζ".into()),
        ("eta".into(), "η".into()),
        ("theta".into(), "θ".into()),
        ("vartheta".into(), "ϑ".into()),
        ("iota".into(), "ι".into()),
        ("kappa".into(), "κ".into()),
        ("lambda".into(), "λ".into()),
        ("mu".into(), "μ".into()),
        ("nu".into(), "ν".into()),
        ("xi".into(), "ξ".into()),
        ("pi".into(), "π".into()),
        ("varpi".into(), "ϖ".into()),
        ("rho".into(), "ρ".into()),
        ("varrho".into(), "ϱ".into()),
        ("sigma".into(), "σ".into()),
        ("varsigma".into(), "ς".into()),
        ("tau".into(), "τ".into()),
        ("upsilon".into(), "υ".into()),
        ("phi".into(), "φ".into()),
        ("varphi".into(), "ϕ".into()),
        ("chi".into(), "χ".into()),
        ("psi".into(), "ψ".into()),
        ("omega".into(), "ω".into()),
        ("Delta".into(), "Δ".into()),
        ("Theta".into(), "Θ".into()),
        ("Lambda".into(), "Λ".into()),
        ("Xi".into(), "Ξ".into()),
        ("Pi".into(), "Π".into()),
        ("Sigma".into(), "Σ".into()),
        ("Upsilon".into(), "Υ".into()),
        ("Phi".into(), "Φ".into()),
        ("Psi".into(), "Ψ".into()),
        ("Omega".into(), "Ω".into()),
    ])
}
