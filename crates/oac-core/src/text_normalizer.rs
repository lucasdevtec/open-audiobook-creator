use regex::{Captures, Regex};
use std::sync::LazyLock;

/// Converte números inteiros (0 a 999.999.999.999) em palavras em português.
pub fn integer_to_words_pt(n: u64) -> String {
    if n == 0 {
        return "zero".to_string();
    }

    let unidades = [
        "", "um", "dois", "três", "quatro", "cinco", "seis", "sete", "oito", "nove",
    ];
    let de_10_a_19 = [
        "dez", "onze", "doze", "treze", "quatorze", "quinze", "dezesseis", "dezessete", "dezoito",
        "dezenove",
    ];
    let dezenas = [
        "", "", "vinte", "trinta", "quarenta", "cinquenta", "sessenta", "setenta", "oitenta",
        "noventa",
    ];
    let centenas = [
        "",
        "cento",
        "duzentos",
        "trezentos",
        "quatrocentos",
        "quinhentos",
        "seiscentos",
        "setecentos",
        "oitocentos",
        "novecentos",
    ];

    fn centena_para_palavras(
        num: u64,
        unidades: &[&str],
        de_10_a_19: &[&str],
        dezenas: &[&str],
        centenas: &[&str],
    ) -> String {
        if num == 100 {
            return "cem".to_string();
        }

        let c = (num / 100) as usize;
        let d = ((num % 100) / 10) as usize;
        let u = (num % 10) as usize;

        let mut partes = Vec::new();

        if c > 0 {
            partes.push(centenas[c].to_string());
        }

        if d == 1 {
            partes.push(de_10_a_19[u].to_string());
        } else {
            if d > 1 {
                partes.push(dezenas[d].to_string());
            }
            if u > 0 {
                partes.push(unidades[u].to_string());
            }
        }

        partes.join(" e ")
    }

    let bilhoes = n / 1_000_000_000;
    let milhoes = (n % 1_000_000_000) / 1_000_000;
    let milhares = (n % 1_000_000) / 1_000;
    let resto = n % 1_000;

    let mut result = String::new();

    if bilhoes > 0 {
        let texto = centena_para_palavras(bilhoes, &unidades, &de_10_a_19, &dezenas, &centenas);
        if bilhoes == 1 {
            result.push_str(&format!("{texto} bilhão"));
        } else {
            result.push_str(&format!("{texto} bilhões"));
        }
    }

    if milhoes > 0 {
        if !result.is_empty() {
            result.push_str(" e ");
        }
        let texto = centena_para_palavras(milhoes, &unidades, &de_10_a_19, &dezenas, &centenas);
        if milhoes == 1 {
            result.push_str(&format!("{texto} milhão"));
        } else {
            result.push_str(&format!("{texto} milhões"));
        }
    }

    if milhares > 0 {
        if !result.is_empty() {
            result.push_str(" e ");
        }
        if milhares == 1 {
            result.push_str("mil");
        } else {
            let texto = centena_para_palavras(milhares, &unidades, &de_10_a_19, &dezenas, &centenas);
            result.push_str(&format!("{texto} mil"));
        }
    }

    if resto > 0 {
        if !result.is_empty() {
            // Em português: usa-se "e" se for centena redonda (100, 200...) ou menor que 100 (1..99).
            // Se tiver centena + dezena/unidade (ex: 984), usa-se espaço direto: "mil novecentos e oitenta e quatro".
            if resto % 100 == 0 || resto < 100 {
                result.push_str(" e ");
            } else {
                result.push(' ');
            }
        }
        let texto = centena_para_palavras(resto, &unidades, &de_10_a_19, &dezenas, &centenas);
        result.push_str(&texto);
    }

    result
}

/// Converte ordinais (1º a 99º) em português.
pub fn ordinal_to_words_pt(n: u64, feminino: bool) -> Option<String> {
    if n == 0 || n > 99 {
        return None;
    }

    let ord_unidades_m = [
        "", "primeiro", "segundo", "terceiro", "quarto", "quinto", "sexto", "sétimo", "oitavo",
        "nono",
    ];
    let ord_unidades_f = [
        "", "primeira", "segunda", "terceira", "quarta", "quinta", "sexta", "sétima", "oitava",
        "nona",
    ];
    let ord_dezenas_m = [
        "",
        "décimo",
        "vigésimo",
        "trigésimo",
        "quadragésimo",
        "quinquagésimo",
        "sexagésimo",
        "septuagésimo",
        "octogésimo",
        "nonagésimo",
    ];
    let ord_dezenas_f = [
        "",
        "décima",
        "vigésima",
        "trigésima",
        "quadragésima",
        "quinquagésima",
        "sexagésima",
        "septuagésima",
        "octogésima",
        "nonagésima",
    ];

    let (u_arr, d_arr) = if feminino {
        (&ord_unidades_f, &ord_dezenas_f)
    } else {
        (&ord_unidades_m, &ord_dezenas_m)
    };

    let d = (n / 10) as usize;
    let u = (n % 10) as usize;

    let mut partes = Vec::new();
    if d > 0 {
        partes.push(d_arr[d]);
    }
    if u > 0 {
        partes.push(u_arr[u]);
    }

    Some(partes.join(" "))
}

/// Converte algarismos romanos (I a XXX) para cardinais ou ordinais em português.
pub fn roman_to_int(roman: &str) -> Option<u64> {
    match roman.to_ascii_uppercase().as_str() {
        "I" => Some(1),
        "II" => Some(2),
        "III" => Some(3),
        "IV" => Some(4),
        "V" => Some(5),
        "VI" => Some(6),
        "VII" => Some(7),
        "VIII" => Some(8),
        "IX" => Some(9),
        "X" => Some(10),
        "XI" => Some(11),
        "XII" => Some(12),
        "XIII" => Some(13),
        "XIV" => Some(14),
        "XV" => Some(15),
        "XVI" => Some(16),
        "XVII" => Some(17),
        "XVIII" => Some(18),
        "XIX" => Some(19),
        "XX" => Some(20),
        "XXI" => Some(21),
        "XXII" => Some(22),
        "XXIII" => Some(23),
        "XXIV" => Some(24),
        "XXV" => Some(25),
        "XXX" => Some(30),
        _ => None,
    }
}

// Regexes estáticas compiladas uma única vez
static RE_MOEDA_BRL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)R\$\s*(\d{1,3}(?:\.\d{3})*|\d+)(?:,(\d{1,2}))?").unwrap()
});

static RE_MOEDA_USD_EUR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([$€]|US\$)\s*(\d{1,3}(?:\.\d{3})*|\d+)(?:,(\d{1,2}))?").unwrap()
});

static RE_PORCENTAGEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d+(?:,\d+)?)\s*%").unwrap()
});

static RE_CAPITULO_ROMANO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(cap[íi]tulo|livro|ato|cena|parte)\s+([IVXLCDM]+)\b").unwrap()
});

static RE_SECULOS_ROMANOS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(s[eé]culo|sec\.|séc\.)\s+([IVXLCDM]+)\b").unwrap()
});

static RE_NOBRES_ROMANOS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(dom|d\.|papa|rei|rainha)\s+([A-ZÁÉÍÓÚÂÊÎÔÛÃÕa-záéíóúâêîôûãõ]+(?:\s+[A-ZÁÉÍÓÚÂÊÎÔÛÃÕa-záéíóúâêîôûãõ]+)?)\s+([IVXLCDM]+)\b").unwrap()
});

static RE_ANO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(em|ano\s+de|desde|at[eé]|de)\s+([12]\d{3})\b").unwrap()
});

static RE_ORDINAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d{1,2})([ºª°])").unwrap()
});

static RE_NUMERO_DECIMAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d+),(\d+)\b").unwrap()
});

static RE_NUMERO_INTEIRO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{1,3}(?:\.\d{3})+|\d+)\b").unwrap()
});

/// Normaliza um texto em português brasileiro para narração fluida e natural em áudio.
pub fn normalize_portuguese_text(input: &str) -> String {
    let mut text = input.to_string();

    // 1. Substituir abreviaturas literárias comuns
    text = Regex::new(r"(?i)\bCap\.\s*").unwrap().replace_all(&text, "Capítulo ").to_string();
    text = Regex::new(r"(?i)\bp[áa]g[s]?\.\s*(\d+)").unwrap().replace_all(&text, "página $1").to_string();
    text = Regex::new(r"(?i)\bDr\.\s+").unwrap().replace_all(&text, "Doutor ").to_string();
    text = Regex::new(r"(?i)\bDra\.\s+").unwrap().replace_all(&text, "Doutora ").to_string();
    text = Regex::new(r"(?i)\bSr\.\s+").unwrap().replace_all(&text, "Senhor ").to_string();
    text = Regex::new(r"(?i)\bSra\.\s+").unwrap().replace_all(&text, "Senhora ").to_string();
    text = Regex::new(r"(?i)\bProf\.\s+").unwrap().replace_all(&text, "Professor ").to_string();
    text = Regex::new(r"(?i)\bProfa\.\s+").unwrap().replace_all(&text, "Professora ").to_string();
    text = Regex::new(r"(?i)\betc\.").unwrap().replace_all(&text, "etcétera").to_string();

    // 2. Títulos com algarismos romanos: "Capítulo IV" -> "Capítulo Quatro"
    text = RE_CAPITULO_ROMANO.replace_all(&text, |caps: &Captures| {
        let prefixo = &caps[1];
        let roman = &caps[2];
        if let Some(val) = roman_to_int(roman) {
            let words = integer_to_words_pt(val);
            let capitalized = capitalize_first(&words);
            format!("{prefixo} {capitalized}")
        } else {
            caps[0].to_string()
        }
    }).to_string();

    // 3. Séculos com romanos: "Século XXI" -> "Século vinte e um", "Século IV" -> "Século quatro"
    text = RE_SECULOS_ROMANOS.replace_all(&text, |caps: &Captures| {
        let prefixo = &caps[1];
        let roman = &caps[2];
        if let Some(val) = roman_to_int(roman) {
            let words = integer_to_words_pt(val);
            format!("{prefixo} {words}")
        } else {
            caps[0].to_string()
        }
    }).to_string();

    // 3b. Nomes históricos / nobres: "Dom Pedro II" -> "Dom Pedro Segundo", "Papa João Paulo II" -> "Papa João Paulo Segundo"
    text = RE_NOBRES_ROMANOS.replace_all(&text, |caps: &Captures| {
        let titulo = &caps[1];
        let nome = &caps[2];
        let roman = &caps[3];
        if let Some(val) = roman_to_int(roman) {
            let words = ordinal_to_words_pt(val, false).unwrap_or_else(|| integer_to_words_pt(val));
            let capitalized = capitalize_first(&words);
            format!("{titulo} {nome} {capitalized}")
        } else {
            caps[0].to_string()
        }
    }).to_string();

    // 4. Moedas: "R$ 50,00" -> "cinquenta reais", "R$ 1,50" -> "um real e cinquenta centavos"
    text = RE_MOEDA_BRL.replace_all(&text, |caps: &Captures| {
        let raw_reais = caps[1].replace('.', "");
        let centavos_str = caps.get(2).map(|m| m.as_str()).unwrap_or("00");
        let reais: u64 = raw_reais.parse().unwrap_or(0);
        let centavos: u64 = centavos_str.parse().unwrap_or(0);

        let mut partes = Vec::new();
        if reais > 0 {
            let reais_words = integer_to_words_pt(reais);
            if reais == 1 {
                partes.push("um real".to_string());
            } else {
                partes.push(format!("{reais_words} reais"));
            }
        }
        if centavos > 0 {
            let cent_words = integer_to_words_pt(centavos);
            if centavos == 1 {
                partes.push("um centavo".to_string());
            } else {
                partes.push(format!("{cent_words} centavos"));
            }
        }

        if partes.is_empty() {
            "zero reais".to_string()
        } else {
            partes.join(" e ")
        }
    }).to_string();

    // 5. Moedas estrangeiras ($ e €)
    text = RE_MOEDA_USD_EUR.replace_all(&text, |caps: &Captures| {
        let simbolo = &caps[1];
        let raw_val = caps[2].replace('.', "");
        let val: u64 = raw_val.parse().unwrap_or(0);
        let words = integer_to_words_pt(val);
        let moeda = if simbolo == "€" {
            if val == 1 { "euro" } else { "euros" }
        } else {
            if val == 1 { "dólar" } else { "dólares" }
        };
        format!("{words} {moeda}")
    }).to_string();

    // 6. Porcentagens: "50%" -> "cinquenta por cento", "10,5%" -> "dez vírgula cinco por cento"
    text = RE_PORCENTAGEM.replace_all(&text, |caps: &Captures| {
        let num_str = &caps[1];
        if num_str.contains(',') {
            let parts: Vec<&str> = num_str.split(',').collect();
            let int_p: u64 = parts[0].parse().unwrap_or(0);
            let dec_p: u64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            format!(
                "{} vírgula {} por cento",
                integer_to_words_pt(int_p),
                integer_to_words_pt(dec_p)
            )
        } else {
            let val: u64 = num_str.parse().unwrap_or(0);
            format!("{} por cento", integer_to_words_pt(val))
        }
    }).to_string();

    // 7. Anos contextuais: "em 1984" -> "em mil novecentos e oitenta e quatro"
    text = RE_ANO.replace_all(&text, |caps: &Captures| {
        let prefixo = &caps[1];
        let ano: u64 = caps[2].parse().unwrap_or(0);
        format!("{prefixo} {}", integer_to_words_pt(ano))
    }).to_string();

    // 8. Ordinais: "1º" -> "primeiro", "2ª" -> "segunda"
    text = RE_ORDINAL.replace_all(&text, |caps: &Captures| {
        let n: u64 = caps[1].parse().unwrap_or(0);
        let ind = &caps[2];
        let fem = ind == "ª";
        ordinal_to_words_pt(n, fem).unwrap_or_else(|| caps[0].to_string())
    }).to_string();

    // 9. Decimais gerais: "3,14" -> "três vírgula quatorze"
    text = RE_NUMERO_DECIMAL.replace_all(&text, |caps: &Captures| {
        let int_p: u64 = caps[1].parse().unwrap_or(0);
        let dec_p: u64 = caps[2].parse().unwrap_or(0);
        format!(
            "{} vírgula {}",
            integer_to_words_pt(int_p),
            integer_to_words_pt(dec_p)
        )
    }).to_string();

    // 10. Números inteiros gerais soltos no texto: "42" -> "quarenta e dois"
    // Evitamos números já convertidos ou dentro de palavras
    text = RE_NUMERO_INTEIRO.replace_all(&text, |caps: &Captures| {
        let raw = caps[1].replace('.', "");
        if let Ok(n) = raw.parse::<u64>() {
            // Limitar para números até 1 bilhão para segurança
            if n <= 1_000_000_000 {
                return integer_to_words_pt(n);
            }
        }
        caps[0].to_string()
    }).to_string();

    // 11. Normalizar travessões literais para pausas limpas sem quebras estranhas
    text = text.replace("—", " — ");
    text = text.replace("–", " – ");
    text = Regex::new(r"\s+").unwrap().replace_all(&text, " ").to_string();

    text.trim().to_string()
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integers_to_words() {
        assert_eq!(integer_to_words_pt(0), "zero");
        assert_eq!(integer_to_words_pt(1), "um");
        assert_eq!(integer_to_words_pt(16), "dezesseis");
        assert_eq!(integer_to_words_pt(42), "quarenta e dois");
        assert_eq!(integer_to_words_pt(100), "cem");
        assert_eq!(integer_to_words_pt(105), "cento e cinco");
        assert_eq!(integer_to_words_pt(1000), "mil");
        assert_eq!(integer_to_words_pt(1500), "mil e quinhentos");
        assert_eq!(integer_to_words_pt(1984), "mil novecentos e oitenta e quatro");
        assert_eq!(integer_to_words_pt(2026), "dois mil e vinte e seis");
    }

    #[test]
    fn test_text_normalization() {
        let input = "Em 1984, na pág. 25 do Cap. IV, ele pagou R$ 50,00 com 10% de desconto.";
        let norm = normalize_portuguese_text(input);
        assert!(norm.contains("mil novecentos e oitenta e quatro"));
        assert!(norm.contains("página vinte e cinco"));
        assert!(norm.contains("Capítulo Quatro"));
        assert!(norm.contains("cinquenta reais"));
        assert!(norm.contains("dez por cento"));
    }

    #[test]
    fn test_roman_centuries_and_kings() {
        let input = "No século XXI, governado por Dom Pedro II.";
        let norm = normalize_portuguese_text(input);
        assert!(norm.contains("século Vinte e um") || norm.contains("Século Vinte e um") || norm.contains("vinte e um"));
        assert!(norm.contains("Dom Pedro Segundo"));
    }
}
