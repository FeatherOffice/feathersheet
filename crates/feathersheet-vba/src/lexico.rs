//! O léxico do VBA: o código-fonte vira fichas com o número da linha. Maiúsculas não
//! importam no VBA (`dim` = `Dim`); quem compara palavra-chave é a sintaxe, sem
//! diferenciar. A continuação de linha (` _` no fim) some aqui; o `:` que separa
//! instruções vira [`Ficha::Sep`].

/// Uma ficha do léxico.
#[derive(Clone, Debug, PartialEq)]
pub enum Ficha {
    /// Identificador ou palavra-chave, como escrito (sem o sufixo de tipo `$`, `%`, `&`).
    Nome(String),
    /// Número: o valor e o texto original (`&HFF`, `1.5`, `3#`).
    Num(f64),
    /// Texto entre aspas, já sem as aspas e com `""` virado `"`.
    Texto(String),
    /// Data entre cerquilhas (`#1/31/2024#`), como escrita.
    Data(String),
    /// Operador ou pontuação: `=`, `<>`, `:=`, `(`, `.`, `&`, `#`…
    Op(&'static str),
    /// Quebra de linha.
    Fim,
    /// `:` separando instruções na mesma linha (o `If … Then a: b` de uma linha precisa
    /// saber a diferença para a quebra de linha).
    Sep,
}

/// Uma ficha e a linha (base 1) onde ela começa.
#[derive(Clone, Debug, PartialEq)]
pub struct Tok {
    pub f: Ficha,
    pub linha: usize,
}

const OPERADORES: [&str; 21] = [":=", "<>", "<=", ">=", "=", "<", ">", "+", "-", "*", "/", "\\", "^", "&", "(", ")", ",", ".", "!", ";", "#"];

fn comeca_nome(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn continua_nome(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

struct Lexico<'a> {
    c: Vec<char>,
    i: usize,
    linha: usize,
    saida: Vec<Tok>,
    _fonte: &'a str,
}

impl Lexico<'_> {
    fn olha(&self, d: usize) -> Option<char> {
        self.c.get(self.i + d).copied()
    }

    fn por(&mut self, f: Ficha) {
        self.saida.push(Tok { f, linha: self.linha });
    }

    fn fim_de_instrucao(&mut self) {
        match self.saida.last() {
            None | Some(Tok { f: Ficha::Fim, .. }) => {}
            // `a: ⏎` — o `:` no fim da linha não separa nada.
            Some(Tok { f: Ficha::Sep, .. }) => self.saida.last_mut().expect("conferido").f = Ficha::Fim,
            _ => self.por(Ficha::Fim),
        }
    }

    /// `' comentário` e `Rem comentário`: pula até a quebra de linha.
    fn pular_linha(&mut self) {
        while let Some(c) = self.olha(0) {
            if c == '\n' {
                break;
            }
            self.i += 1;
        }
    }

    /// ` _` no fim da linha: a instrução continua na de baixo.
    fn continuacao(&mut self) -> bool {
        if self.olha(0) != Some('_') || self.olha(1).is_some_and(continua_nome) {
            return false;
        }
        let mut j = self.i + 1;
        while matches!(self.c.get(j), Some(' ' | '\t' | '\r')) {
            j += 1;
        }
        if matches!(self.c.get(j), Some('\n') | None) {
            self.i = j + 1;
            self.linha += 1;
            return true;
        }
        false
    }

    fn texto(&mut self) {
        self.i += 1;
        let mut s = String::new();
        while let Some(c) = self.olha(0) {
            self.i += 1;
            if c == '"' {
                if self.olha(0) == Some('"') {
                    s.push('"');
                    self.i += 1;
                    continue;
                }
                break;
            }
            if c == '\n' {
                // Aspas sem fechar: o VBA não aceita; fecha aqui para não engolir o arquivo.
                self.i -= 1;
                break;
            }
            s.push(c);
        }
        self.por(Ficha::Texto(s));
    }

    fn numero(&mut self) {
        let inicio = self.i;
        if self.olha(0) == Some('&') {
            let base = if self.olha(1).is_some_and(|c| c.eq_ignore_ascii_case(&'h')) { 16 } else { 8 };
            self.i += 2;
            let ini = self.i;
            while self.olha(0).is_some_and(|c| c.is_digit(base)) {
                self.i += 1;
            }
            let txt: String = self.c[ini..self.i].iter().collect();
            if self.olha(0) == Some('&') {
                self.i += 1;
            }
            let v = i64::from_str_radix(&txt, base).unwrap_or(0);
            // &HFFFF sem `&` no fim é um Integer de 16 bits (com sinal), como no VBA.
            let v = if base == 16 && txt.len() <= 4 && v > 0x7fff { v - 0x10000 } else { v };
            self.por(Ficha::Num(v as f64));
            return;
        }
        while self.olha(0).is_some_and(|c| c.is_ascii_digit() || c == '.') {
            self.i += 1;
        }
        if self.olha(0).is_some_and(|c| c == 'e' || c == 'E') && self.olha(1).is_some_and(|c| c.is_ascii_digit() || c == '-' || c == '+') {
            self.i += 2;
            while self.olha(0).is_some_and(|c| c.is_ascii_digit()) {
                self.i += 1;
            }
        }
        let txt: String = self.c[inicio..self.i].iter().collect();
        if self.olha(0).is_some_and(|c| matches!(c, '#' | '!' | '@' | '%' | '&' | '^')) && !self.olha(1).is_some_and(continua_nome) {
            self.i += 1;
        }
        self.por(Ficha::Num(txt.parse().unwrap_or(0.0)));
    }

    /// `#1/31/2024#` é data; o `#` de arquivo (`#1`) é operador.
    fn data(&mut self) -> bool {
        let mut j = self.i + 1;
        let mut tem_separador = false;
        while let Some(&c) = self.c.get(j) {
            match c {
                '#' => break,
                '\n' => return false,
                '/' | ':' | '-' => tem_separador = true,
                _ => {}
            }
            j += 1;
        }
        if !tem_separador || self.c.get(j) != Some(&'#') {
            return false;
        }
        let s: String = self.c[self.i + 1..j].iter().collect();
        self.i = j + 1;
        self.por(Ficha::Data(s.trim().to_string()));
        true
    }

    fn nome(&mut self) {
        let inicio = self.i;
        while self.olha(0).is_some_and(continua_nome) {
            self.i += 1;
        }
        let nome: String = self.c[inicio..self.i].iter().collect();
        // Sufixo de tipo colado ao nome: Mid$, x%, n& (o `&` operador vem com espaço).
        if self.olha(0).is_some_and(|c| matches!(c, '$' | '%' | '&' | '!' | '@' | '#')) && !self.olha(1).is_some_and(continua_nome) {
            let sufixo = self.olha(0);
            // `a!b` (acesso por nome) fica como operador.
            if sufixo != Some('!') || !self.olha(1).is_some_and(comeca_nome) {
                self.i += 1;
            }
        }
        // `Rem` no começo da instrução é comentário.
        if nome.eq_ignore_ascii_case("rem") && matches!(self.saida.last(), None | Some(Tok { f: Ficha::Fim | Ficha::Sep, .. })) {
            self.pular_linha();
            return;
        }
        self.por(Ficha::Nome(nome));
    }

    fn operador(&mut self) -> bool {
        for op in OPERADORES {
            let n = op.chars().count();
            if self.c[self.i..].iter().take(n).copied().eq(op.chars()) {
                self.i += n;
                self.por(Ficha::Op(op));
                return true;
            }
        }
        if self.olha(0) == Some(':') {
            self.i += 1;
            // `rotulo:` no começo da instrução fica como `:`; fora disso, separa instruções.
            let n = self.saida.len();
            // `Else:` e afins não são rótulo: o `:` ali só separa (o editor do Excel aceita).
            let palavra_chave = matches!(&self.saida.get(n.wrapping_sub(1)).map(|t| &t.f), Some(Ficha::Nome(p)) if ["else", "then", "loop", "next", "wend", "do", "end"].iter().any(|k| p.eq_ignore_ascii_case(k)));
            let rotulo = n >= 1 && !palavra_chave && matches!(self.saida[n - 1].f, Ficha::Nome(_)) && (n == 1 || self.saida[n - 2].f == Ficha::Fim);
            if rotulo {
                self.por(Ficha::Op(":"));
                self.por(Ficha::Fim);
            } else if !matches!(self.saida.last(), None | Some(Tok { f: Ficha::Fim | Ficha::Sep, .. })) {
                self.por(Ficha::Sep);
            }
            return true;
        }
        false
    }

    fn passo(&mut self) {
        let Some(c) = self.olha(0) else { return };
        match c {
            '\n' => {
                self.fim_de_instrucao();
                self.linha += 1;
                self.i += 1;
            }
            ' ' | '\t' | '\r' => self.i += 1,
            '\'' => self.pular_linha(),
            '"' => self.texto(),
            '_' if self.continuacao() => {}
            '&' if self.olha(1).is_some_and(|d| matches!(d, 'h' | 'H' | 'o' | 'O')) && self.olha(2).is_some_and(|d| d.is_ascii_hexdigit()) => self.numero(),
            c if c.is_ascii_digit() || (c == '.' && self.olha(1).is_some_and(|d| d.is_ascii_digit())) => self.numero(),
            '#' if self.data() => {}
            c if comeca_nome(c) => self.nome(),
            '[' => {
                // [Nome Com Espaço]: identificador entre colchetes.
                let fim = self.c[self.i..].iter().position(|&x| x == ']' || x == '\n').map_or(self.c.len(), |p| self.i + p);
                let nome: String = self.c[self.i + 1..fim].iter().collect();
                self.i = (fim + 1).min(self.c.len());
                self.por(Ficha::Nome(nome));
            }
            _ if self.operador() => {}
            _ => self.i += 1,
        }
    }
}

/// O código VBA em fichas. As linhas `Attribute VB_…` (cabeçalho do módulo) ficam fora.
pub fn fichas(fonte: &str) -> Vec<Tok> {
    let mut l = Lexico { c: fonte.chars().collect(), i: 0, linha: 1, saida: Vec::new(), _fonte: fonte };
    while l.i < l.c.len() {
        l.passo();
    }
    l.fim_de_instrucao();
    l.saida
}

#[cfg(test)]
mod testes {
    use super::*;

    fn so(f: &str) -> Vec<Ficha> {
        fichas(f).into_iter().map(|t| t.f).collect()
    }

    #[test]
    fn texto_com_aspas_dobradas_comentario_e_continuacao() {
        let f = so("x = \"a\"\"b\" ' nada\ny = 1 + _\n  2");
        assert_eq!(f, vec![
            Ficha::Nome("x".into()), Ficha::Op("="), Ficha::Texto("a\"b".into()), Ficha::Fim,
            Ficha::Nome("y".into()), Ficha::Op("="), Ficha::Num(1.0), Ficha::Op("+"), Ficha::Num(2.0), Ficha::Fim,
        ]);
    }

    #[test]
    fn sufixos_hexa_dois_pontos_e_acento() {
        let f = so("s$ = Mid$(t, 1): n = &HFF&\nBotão1_Clique");
        assert_eq!(f[0], Ficha::Nome("s".into()));
        assert_eq!(f[2], Ficha::Nome("Mid".into()));
        assert!(f.contains(&Ficha::Num(255.0)));
        assert!(f.contains(&Ficha::Nome("Botão1_Clique".into())));
        assert_eq!(f.iter().filter(|x| **x == Ficha::Fim).count(), 2);
        assert_eq!(f.iter().filter(|x| **x == Ficha::Sep).count(), 1);
    }

    #[test]
    fn concatenacao_com_espaco_e_argumento_nomeado() {
        let f = so("a = b & c\n.Execute Replace:=2");
        assert!(f.contains(&Ficha::Op("&")));
        assert!(f.contains(&Ficha::Op(":=")));
    }
}
