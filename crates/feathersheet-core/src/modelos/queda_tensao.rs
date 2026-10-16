//! Modelo "Queda de tensão" (NBR 5410): quatro abas — Projeto (identificação e
//! parâmetros), Circuitos (tabela de cálculo), Gráfico e Referências.
//!
//! Método (o da planilha de engenharia original): queda por resistividade,
//! `ΔV = k · ρ · L · I · cos φ / S`, com `k = 2` (mono e bifásico) ou `√3`
//! (trifásico) e a corrente `I = P / (n · V · cos φ)`. A reatância é
//! desprezada; a seção mínima é a menor seção comercial que cumpre o limite.

use super::*;

const ACO: Tema = Tema { forte: "#1F3A5F", clara: "#E3EAF3", listra: "#F5F8FB" };
/// Células de entrada: fundo âmbar claro e letra azul-escura, como nas planilhas de engenharia.
const ENTRADA: &str = "#FFF8E1";
const ENTRADA_COR: &str = "#0B3D91";
const CALCULADO: &str = "#F3F4F6";

const SECOES: [&str; 16] = [
    "1,5", "2,5", "4", "6", "10", "16", "25", "35", "50", "70", "95", "120", "150", "185", "240", "300",
];

/// Abas e linhas congeladas (cabeçalho da tabela de circuitos).
pub const CONGELAR: &[(usize, u32, u32)] = &[(1, 7, 0)];

/// Linha do cabeçalho e da primeira linha de dados na aba Circuitos.
const CAB: u32 = R_CABECALHO;
const DADOS: u32 = R_DADOS;
/// Última linha olhada pelos resumos (a tabela pode crescer).
const FIM: u32 = 206;

fn num(dec: f64) -> EstiloParcial {
    estilo().campo("format", Campo::Texto("number".into())).campo("decimals", Campo::Numero(dec)).alinhar("center")
}

fn pct(dec: f64) -> EstiloParcial {
    estilo().campo("format", Campo::Texto("percent".into())).campo("decimals", Campo::Numero(dec)).alinhar("center")
}

fn nome(w: &mut Workbook, n: &str, aba: &str, r0: u32, c0: u32, r1: u32, c1: u32) {
    let mut a = Rect::new(r0, c0, r1, c1);
    a.sheet = Some(aba.to_string());
    w.definir_nome(n, a);
}

/// Uma regra de validação já em JSON (o formato de `operacoes::validation::Validacao`;
/// o core não lê nem escreve JSON, então o texto é montado aqui).
fn validacao(ranges: &[Rect], regra: String, entrada: (&str, &str), erro: &str) -> String {
    let rs: Vec<String> = ranges.iter()
        .map(|r| format!(r#"{{"r0":{},"c0":{},"r1":{},"c1":{}}}"#, r.r0, r.c0, r.r1, r.c1)).collect();
    format!(
        r#"{{"ranges":[{}],"regra":{regra},"quando":"recusar","mensagem":{},"mensagens":{{"entrada_titulo":{},"entrada_texto":{},"erro_titulo":"Valor inválido"}}}}"#,
        rs.join(","), json_texto(erro), json_texto(entrada.0), json_texto(entrada.1)
    )
}

fn lista(ranges: &[Rect], valores: &[&str], entrada: (&str, &str)) -> String {
    let v: Vec<String> = valores.iter().map(|s| json_texto(s)).collect();
    validacao(ranges, format!(r#"{{"tipo":"lista","valores":[{}]}}"#, v.join(",")), entrada, "Escolha um dos valores da lista.")
}

fn entre(ranges: &[Rect], a: f64, b: f64, entrada: (&str, &str), erro: &str) -> String {
    validacao(ranges, format!(r#"{{"tipo":"numero","op":"entre","a":{a},"b":{b}}}"#), entrada, erro)
}

fn maior_que_zero(ranges: &[Rect], entrada: (&str, &str), erro: &str) -> String {
    validacao(ranges, r#"{"tipo":"numero","op":"maior","a":0}"#.to_string(), entrada, erro)
}

pub fn criar() -> Workbook {
    let mut w = Workbook::vazia();
    for aba in ["Projeto", "Circuitos", "Balanceamento", "Gráfico", "Referências"] {
        w.criar_aba(aba);
    }
    // Nomes: parâmetros (aba Projeto, coluna E) e tabelas de referência.
    for (i, n) in ["Material_Cond", "Temp_C", "Limite_Queda", "Limite_Desq", "Rho_Ef"].iter().enumerate() {
        nome(&mut w, n, "Projeto", 4 + i as u32, 4, 4 + i as u32, 4);
    }
    nome(&mut w, "Materiais", "Referências", 4, 0, 5, 0);
    nome(&mut w, "Rho_Base", "Referências", 4, 1, 5, 1);
    nome(&mut w, "Alfa_Coef", "Referências", 4, 2, 5, 2);
    nome(&mut w, "Secoes_mm2", "Referências", 4, 4, 4 + SECOES.len() as u32 - 1, 4);

    w.editar(0, |s| projeto(s));
    w.editar(1, |s| circuitos(s));
    w.editar(2, |s| balanceamento(s));
    w.editar(3, |s| grafico(s));
    w.editar(4, |s| referencias(s));
    w
}

fn montar(s: &mut Sheet, montar: impl FnOnce(&mut Folha)) {
    let mut f = Folha { s, t: ACO, mesclas: Vec::new(), regras: Vec::new() };
    montar(&mut f);
    f.concluir();
}

/// Rótulo e valor mesclados lado a lado.
fn par(f: &mut Folha, r: u32, (l0, l1): (u32, u32), (v0, v1): (u32, u32), rotulo: &str, valor: &str, entrada: bool, fmt: EstiloParcial) {
    f.pintar(r, l0, r, l1, &estilo().fundo(ACO.clara).cor(TEXTO).negrito().borda(&["bottom"], BRANCO));
    f.celula(r, l0, rotulo, estilo());
    f.mesclar(r, l0, r, l1);
    let (fundo, cor) = if entrada { (ENTRADA, ENTRADA_COR) } else { (CALCULADO, TEXTO) };
    let base = estilo().fundo(fundo).cor(cor).borda(&["bottom", "top", "left", "right"], "#C9D3E0");
    let mut e = base;
    for (k, v) in fmt { e.insert(k, v); }
    f.pintar(r, v0, r, v1, &e);
    f.celula(r, v0, valor, estilo());
    f.mesclar(r, v0, r, v1);
    f.altura(r, 30);
}

fn projeto(s: &mut Sheet) {
    montar(s, |f| {
        f.titulo(5, "Queda de tensão — NBR 5410",
            "Identifique o projeto e escolha os parâmetros; circuitos e balanceamento são calculados nas outras abas.");
        f.cabecalho(3, 0, &["PROJETO", ""]);
        f.mesclar(3, 0, 3, 1);
        f.cabecalho(3, 3, &["PARÂMETROS DE CÁLCULO", "", ""]);
        f.mesclar(3, 3, 3, 5);
        let esq = [
            ("Obra", "Galpão industrial — QF-01"),
            ("Cliente", "Metalúrgica Horizonte Ltda."),
            ("Responsável técnico", "Eng. Carlos Mendes"),
            ("CREA", "CREA-SP 5063871290"),
            ("Data", "15/10/2026"),
            ("Revisão", "R0"),
        ];
        for (i, (r, v)) in esq.iter().enumerate() {
            let fmt = if *r == "Data" { estilo().campo("format", Campo::Texto("date".into())).alinhar("left") } else { estilo().alinhar("left") };
            par(f, 4 + i as u32, (0, 0), (1, 1), r, v, true, fmt);
        }
        par(f, 10, (0, 0), (1, 1), "Norma de referência", "ABNT NBR 5410:2004", false, estilo().alinhar("left"));

        let dir: [(&str, &str, bool, EstiloParcial, &str); 5] = [
            ("Material do condutor", "Cobre", true, estilo().alinhar("center"), "cobre / alumínio"),
            ("Temperatura do condutor", "20", true, num(0.0), "°C (20 ou 70)"),
            ("Limite de queda", "0,04", true, pct(1.0), "4% em terminais"),
            ("Limite de desequilíbrio", "0,1", true, pct(1.0), "entre fases R, S e T"),
            ("Resistividade efetiva ρ", "=ÍNDICE(Rho_Base;CORRESP(Material_Cond;Referências!$A$5:$A$6;0))*(1+ÍNDICE(Alfa_Coef;CORRESP(Material_Cond;Referências!$A$5:$A$6;0))*(Temp_C-20))",
                false, num(5.0), "Ω·mm²/m"),
        ];
        for (i, (r, v, ent, fmt, un)) in dir.iter().enumerate() {
            let linha = 4 + i as u32;
            par(f, linha, (3, 3), (4, 4), r, v, *ent, fmt.clone());
            f.celula(linha, 5, un, estilo().cor(APAGADO).italico().tamanho(10.5));
        }
        f.celula(9, 3, "Sistema (mono, bi ou trifásico), fases e tensão são definidos em cada circuito.",
            estilo().cor(APAGADO).italico().tamanho(10.5).quebrar().topo());
        f.mesclar(9, 3, 10, 5);
        let nota = |f: &mut Folha, r: u32, x: &str| {
            f.celula(r, 0, x, estilo().cor(APAGADO).italico().tamanho(10.5).quebrar().topo());
            f.mesclar(r, 0, r, 5);
            f.altura(r, 36);
        };
        nota(f, 12, "Limites da NBR 5410 (item 6.2.7): até 4% nos circuitos terminais; até 7% medidos desde o secundário do transformador MT/BT, somando alimentador e terminal.");
        nota(f, 13, "Células em fundo âmbar são de entrada; as cinzas calculam sozinhas. A resistividade a 20 °C está na aba Referências e é corrigida pela temperatura.");
        f.larguras(&[190, 290, 28, 215, 120, 200]);
    });
    // Validações das entradas.
    let r = |r: u32| [Rect::new(r, 4, r, 4)];
    let v = vec![
        lista(&r(4), &["Cobre", "Alumínio"], ("Material", "Resistividade na aba Referências.")),
        lista(&r(5), &["20", "70"], ("Temperatura", "20 °C (valor da planilha original) ou 70 °C (condutor em carga, mais conservador).")),
        entre(&r(6), 0.005, 0.10, ("Limite de queda", "Digite em porcentagem, de 0,5% a 10% (ex.: 4%)."),
            "Informe um limite entre 0,5% e 10%."),
        entre(&r(7), 0.01, 0.50, ("Limite de desequilíbrio", "Digite em porcentagem, de 1% a 50% (padrão 10%)."),
            "Informe um limite entre 1% e 50%."),
    ];
    s.va = Some(format!("[{}]", v.join(",")));
}

/// Colunas da aba Circuitos (base zero).
const C_SIS: u32 = 2;
const C_FASES: u32 = 3;
const C_TENSAO: u32 = 4;
const C_POT: u32 = 5;
const C_FP: u32 = 6;
const C_CORR: u32 = 7;
const C_COMP: u32 = 8;
const C_SECAO: u32 = 9;
const C_K: u32 = 10;
const C_DV: u32 = 11;
const C_DVP: u32 = 12;
const C_SIT: u32 = 13;
const C_MIN: u32 = 14;
/// Circuitos de exemplo.
const N_EXEMPLO: u32 = 10;

fn circuitos(s: &mut Sheet) {
    // Circuito, descrição, sistema, fases, tensão (V), potência (W), FP, comprimento (m), seção (mm²)
    let dados: [[&str; 9]; N_EXEMPLO as usize] = [
        ["C1", "Iluminação do galpão A", "Monofásico", "R", "127", "2400", "0,92", "25", "4"],
        ["C2", "Iluminação do galpão B", "Monofásico", "S", "127", "2400", "0,92", "28", "4"],
        ["C3", "Tomadas do escritório (TUG)", "Monofásico", "T", "127", "3000", "0,95", "30", "6"],
        ["C4", "Aquecedor elétrico", "Bifásico", "RS", "220", "5400", "1", "25", "6"],
        ["C5", "Bomba d'água 3 cv", "Bifásico", "ST", "220", "2760", "0,82", "38", "2,5"],
        ["C6", "Ar-condicionado central", "Trifásico", "RST", "220", "18000", "0,88", "42", "16"],
        ["C7", "Compressor de ar", "Trifásico", "RST", "220", "11000", "0,85", "65", "16"],
        ["C8", "Exaustores da cobertura", "Trifásico", "RST", "220", "3600", "0,8", "90", "2,5"],
        ["C9", "Portão automático", "Monofásico", "S", "127", "1500", "0,75", "150", "6"],
        ["C10", "Tomadas industriais (TUE)", "Bifásico", "TR", "220", "6000", "0,95", "30", "6"],
    ];
    montar(s, |f| {
        f.titulo(14, "Circuitos — cálculo da queda de tensão",
            "=Material_Cond&\" a \"&Temp_C&\" °C · limite de queda \"&ARRED(Limite_Queda*100;1)&\"% — células âmbar são de entrada, cinzas calculam sozinhas\"");
        let sit = col(C_SIT, DADOS, FIM);
        let dv = col(C_DVP, DADOS, FIM);
        f.indicadores(R_INDICADORES, &[
            (0, 1, "Circuitos", format!("=CONT.VALORES({})", col(0, DADOS, FIM)), Fmt::Inteiro),
            (2, 4, "Maior queda", format!("=SE(CONT.NÚM({dv})=0;0;MÁXIMO({dv}))"), Fmt::Pct),
            (5, 8, "Acima do limite", format!("=CONT.SE({sit};\"Acima do limite\")"), Fmt::Inteiro),
            (9, 11, "Limite de queda", "=Limite_Queda".to_string(), Fmt::Pct),
            (12, 14, "Desequilíbrio de fases", "=Balanceamento!B17".to_string(), Fmt::Pct),
        ]);
        f.se((R_INDICADORES + 1, 5, R_INDICADORES + 1, 5), "greaterThan", "0", None, Some(RUIM.1), true);
        f.cabecalho(CAB, 0, &["Circuito", "Descrição", "Sistema", "Fases", "Tensão (V)", "Potência (W)", "FP (cos φ)",
            "Corrente (A)", "Comprimento (m)", "Seção (mm²)", "k", "ΔV (V)", "ΔV (%)", "Situação", "Seção mínima (mm²)"]);
        f.altura(CAB, 40);
        for r in DADOS..=FIM {
            let n = r + 1;
            let (sis, fas, ten, p, fp, cor, l, sc, k, dvv, dvp) = (
                format!("C{n}"), format!("D{n}"), format!("E{n}"), format!("F{n}"), format!("G{n}"),
                format!("H{n}"), format!("I{n}"), format!("J{n}"), format!("K{n}"), format!("L{n}"), format!("M{n}"));
            let f_k = format!("=SE({sis}=\"\";\"\";SE({sis}=\"Trifásico\";RAIZ(3);2))");
            let f_i = format!("=SE(OU({sis}=\"\";{ten}=\"\";{p}=\"\";{fp}=\"\");\"\";{p}/(SE({sis}=\"Trifásico\";RAIZ(3);1)*{ten}*{fp}))");
            let f_dv = format!("=SE(OU({cor}=\"\";{l}=\"\";{sc}=\"\");\"\";{k}*Rho_Ef*{l}*{cor}*{fp}/{sc})");
            let f_dvp = format!("=SE({dvv}=\"\";\"\";{dvv}/{ten})");
            let valido = format!("SE({sis}=\"Trifásico\";{fas}=\"RST\";SE({sis}=\"Bifásico\";OU({fas}=\"RS\";{fas}=\"ST\";{fas}=\"TR\");OU({fas}=\"R\";{fas}=\"S\";{fas}=\"T\")))");
            let f_sit = format!("=SE({dvp}=\"\";\"\";SE({valido};SE({dvp}<=Limite_Queda;\"OK\";\"Acima do limite\");\"Revisar fases\"))");
            let f_min = format!(
                "=SE({dvv}=\"\";\"\";SEERRO(ÍNDICE(Secoes_mm2;CONT.SE(Referências!$E$5:$E$20;\"<\"&({k}*Rho_Ef*{l}*{cor}*{fp}/(Limite_Queda*{ten})))+1);\"Paralelo\"))");
            let i = (r - DADOS) as usize;
            let d = dados.get(i).copied().unwrap_or([""; 9]);
            let entra = |e: EstiloParcial| e.fundo(ENTRADA).cor(ENTRADA_COR).borda(&["bottom"], LINHA);
            let calc = |e: EstiloParcial| e.fundo(CALCULADO).cor(TEXTO).borda(&["bottom"], LINHA);
            f.celula(r, 0, d[0], entra(estilo().alinhar("center").negrito()));
            f.celula(r, 1, d[1], entra(estilo()));
            f.celula(r, C_SIS, d[2], entra(estilo().alinhar("center")));
            f.celula(r, C_FASES, d[3], entra(estilo().alinhar("center")));
            f.celula(r, C_TENSAO, d[4], entra(num(0.0)));
            f.celula(r, C_POT, d[5], entra(estilo().campo("format", Campo::Texto("integer".into())).alinhar("center")));
            f.celula(r, C_FP, d[6], entra(num(2.0)));
            f.celula(r, C_CORR, &f_i, calc(num(1.0)));
            f.celula(r, C_COMP, d[7], entra(num(0.0)));
            f.celula(r, C_SECAO, d[8], entra(num(1.0)));
            f.celula(r, C_K, &f_k, calc(num(3.0)));
            f.celula(r, C_DV, &f_dv, calc(num(2.0)));
            f.celula(r, C_DVP, &f_dvp, calc(pct(2.0)));
            f.celula(r, C_SIT, &f_sit, calc(estilo().alinhar("center").negrito()));
            f.celula(r, C_MIN, &f_min, calc(num(1.0).negrito()));
            f.altura(r, if i < dados.len() { 28 } else { 24 });
        }
        f.situacao((DADOS, C_SIT, FIM, C_SIT), "OK", BOM);
        f.situacao((DADOS, C_SIT, FIM, C_SIT), "Acima do limite", RUIM);
        f.situacao((DADOS, C_SIT, FIM, C_SIT), "Revisar fases", ATENCAO);
        f.se((DADOS, C_DVP, FIM, C_DVP), "formula", "=E(ÉNÚM(M8);M8>Limite_Queda)", None, Some(RUIM.1), true);
        f.barra((DADOS, C_DVP, FIM, C_DVP), "#8FB0D6");
        f.larguras(&[85, 200, 100, 70, 95, 115, 95, 100, 135, 115, 55, 85, 95, 135, 135]);
    });
    // Tabela estruturada sobre o cabeçalho e os circuitos de exemplo.
    let tb = format!(
        r#"[{{"id":"tab_Circuitos","nome":"TabelaCircuitos","area":{{"r0":{CAB},"c0":0,"r1":{},"c1":14}},"temCabecalho":true,"faixas":false,"estilo":{{"tema":"azul","faixasColuna":false,"borda":true}},"filtros":[]}}]"#,
        DADOS + N_EXEMPLO - 1);
    s.tb = Some(tb);
    let col_r = |c: u32| [Rect::new(DADOS, c, FIM, c)];
    let v = vec![
        lista(&col_r(C_SIS), &["Monofásico", "Bifásico", "Trifásico"],
            ("Sistema", "Monofásico e bifásico usam k = 2; trifásico usa k = raiz de 3. Tensão: entre os condutores energizados.")),
        lista(&col_r(C_FASES), &["R", "S", "T", "RS", "ST", "TR", "RST"],
            ("Fases", "Monofásico: R, S ou T. Bifásico: RS, ST ou TR. Trifásico: RST.")),
        lista(&col_r(C_TENSAO), &["127", "220", "380"], ("Tensão (V)", "127, 220 ou 380 V.")),
        maior_que_zero(&col_r(C_POT), ("Potência (W)", "Potência ativa do circuito, em watts."), "A potência deve ser maior que zero."),
        entre(&col_r(C_FP), 0.1, 1.0, ("Fator de potência", "cos φ entre 0,10 e 1,00."), "O fator de potência fica entre 0,10 e 1,00."),
        maior_que_zero(&col_r(C_COMP), ("Comprimento (m)", "Distância do quadro à carga, em metros."), "O comprimento deve ser maior que zero."),
        validacao(&col_r(C_SECAO), r#"{"tipo":"formula","formula":"=Secoes_mm2"}"#.to_string(),
            ("Seção (mm²)", "Escolha uma seção comercial (aba Referências)."), "Escolha uma seção comercial da lista."),
    ];
    s.va = Some(format!("[{}]", v.join(",")));
}

fn balanceamento(s: &mut Sheet) {
    const D: &str = "Circuitos!$D$8:$D$207";
    const F: &str = "Circuitos!$F$8:$F$207";
    const H: &str = "Circuitos!$H$8:$H$207";
    montar(s, |f| {
        f.titulo(3, "Balanceamento de carga entre fases",
            "Soma dos circuitos em R, S e T — o desequilíbrio é (máxima - média) / média.");
        f.indicadores(R_INDICADORES, &[
            (0, 0, "Desequilíbrio (A)", "=B17".to_string(), Fmt::Pct),
            (1, 1, "Situação", "=B18".to_string(), Fmt::Texto),
            (2, 2, "Fase mais carregada", "=ÍNDICE(A8:A10;CORRESP(MÁXIMO(B8:B10);B8:B10;0))".to_string(), Fmt::Texto),
            (3, 3, "Limite", "=Limite_Desq".to_string(), Fmt::Pct),
        ]);
        f.situacao((R_INDICADORES + 1, 1, R_INDICADORES + 1, 1), "OK", BOM);
        f.situacao((R_INDICADORES + 1, 1, R_INDICADORES + 1, 1), "Desequilibrado", RUIM);
        f.cabecalho(CAB, 0, &["Fase", "Corrente (A)", "Carga (W)", "Desvio da média (A)"]);
        let dados: Vec<Vec<String>> = ["R", "S", "T"].iter().enumerate().map(|(i, ph)| {
            let n = DADOS + i as u32 + 1;
            let (a, b) = match *ph { "R" => ("RS", "TR"), "S" => ("RS", "ST"), _ => ("ST", "TR") };
            vec![ph.to_string(),
                format!("=SOMASE({D};\"{ph}\";{H})+SOMASE({D};\"{a}\";{H})+SOMASE({D};\"{b}\";{H})+SOMASE({D};\"RST\";{H})"),
                format!("=SOMASE({D};\"{ph}\";{F})+(SOMASE({D};\"{a}\";{F})+SOMASE({D};\"{b}\";{F}))/2+SOMASE({D};\"RST\";{F})/3"),
                format!("=SE($B$14=0;0;(B{n}-$B$14)/$B$14)")]
        }).collect();
        f.linhas(DADOS, 0, &[Fmt::Centro, Fmt::Nota, Fmt::Inteiro, Fmt::Pct], &dados);
        f.total(10, 0, &[("Total", Fmt::Centro), ("=SOMA(B8:B10)", Fmt::Nota), ("=SOMA(C8:C10)", Fmt::Inteiro), ("", Fmt::Texto)]);
        f.cabecalho(12, 0, &["Resumo", "Corrente (A)", "Carga (W)", "Critério"]);
        let resumo = [
            ("Média", "=MÉDIA(B8:B10)", "=MÉDIA(C8:C10)", ""),
            ("Máxima", "=MÁXIMO(B8:B10)", "=MÁXIMO(C8:C10)", ""),
            ("Mínima", "=MÍNIMO(B8:B10)", "=MÍNIMO(C8:C10)", ""),
            ("Desequilíbrio", "=SE(B14=0;0;(B15-B14)/B14)", "=SE(C14=0;0;(C15-C14)/C14)", "(máx - média) / média"),
        ];
        for (i, (rot, a, w, c)) in resumo.iter().enumerate() {
            let r = 13 + i as u32;
            let fmt = if i == 3 { pct(2.0) } else { num(1.0) };
            let fmtw = if i == 3 { pct(2.0) } else { estilo().campo("format", Campo::Texto("integer".into())).alinhar("center") };
            let base = |e: EstiloParcial| e.fundo(if i % 2 == 1 { ACO.listra } else { BRANCO }).cor(TEXTO).borda(&["bottom"], LINHA);
            f.celula(r, 0, rot, base(estilo().negrito()));
            f.celula(r, 1, a, base(fmt));
            f.celula(r, 2, w, base(fmtw));
            f.celula(r, 3, c, base(estilo().cor(APAGADO).italico()));
            f.altura(r, ALTURA_LINHA);
        }
        f.celula(17, 0, "Situação", estilo().fundo(ACO.clara).negrito().cor(TEXTO).borda(&["top"], ACO.forte));
        f.celula(17, 1, "=SE(B17<=Limite_Desq;\"OK\";\"Desequilibrado\")", estilo().fundo(ACO.clara).negrito().alinhar("center").borda(&["top"], ACO.forte));
        f.pintar(17, 2, 17, 3, &estilo().fundo(ACO.clara).borda(&["top"], ACO.forte));
        f.celula(17, 3, "=\"limite de \"&ARRED(Limite_Desq*100;1)&\"%\"", estilo().fundo(ACO.clara).cor(APAGADO).italico().borda(&["top"], ACO.forte));
        f.altura(17, ALTURA_LINHA + 2);
        f.situacao((17, 1, 17, 1), "OK", BOM);
        f.situacao((17, 1, 17, 1), "Desequilibrado", RUIM);
        f.se((DADOS, 3, DADOS + 2, 3), "formula", "=ABS(D8)>Limite_Desq", None, Some(RUIM.1), true);
        f.larguras(&[170, 150, 150, 190]);
    });
    let g = r##"[{"id":"grafico_fases","type":"column","title":"Corrente por fase (A)","range":{"r0":6,"c0":0,"r1":9,"c1":1},"seriesInColumns":true,"hasHeaders":true,"hasCategoryLabels":true,"style":{"showTitle":true,"legendPosition":"none","dataLabels":true,"seriesColors":{"0":"#1F3A5F"}},"position":{"x":700.0,"y":130.0,"width":480.0,"height":320.0}}]"##;
    s.g = Some(g.to_string());
}

fn grafico(s: &mut Sheet) {
    montar(s, |f| {
        f.titulo(2, "Queda de tensão por circuito", "Vem da aba Circuitos.");
        f.cabecalho(3, 0, &["Circuito", "ΔV (%)", "Limite (%)"]);
        let dados: Vec<Vec<String>> = (0..N_EXEMPLO).map(|i| {
            let n = DADOS + i + 1;
            vec![format!("=SE(Circuitos!A{n}=\"\";\"\";Circuitos!A{n})"),
                format!("=SE(Circuitos!M{n}=\"\";0;Circuitos!M{n}*100)"), "=Limite_Queda*100".to_string()]
        }).collect();
        f.linhas(4, 0, &[Fmt::Centro, Fmt::Nota, Fmt::Nota], &dados);
        f.larguras(&[120, 120, 120]);
    });
    let g = r##"[{"id":"grafico_queda","type":"column","title":"ΔV (%) por circuito × limite","range":{"r0":3,"c0":0,"r1":13,"c1":2},"seriesInColumns":true,"hasHeaders":true,"hasCategoryLabels":true,"style":{"showTitle":true,"legendPosition":"bottom","dataLabels":false,"seriesColors":{"0":"#1F3A5F","1":"#C62828"}},"position":{"x":400.0,"y":80.0,"width":640.0,"height":380.0}}]"##;
    s.g = Some(g.to_string());
}

fn referencias(s: &mut Sheet) {
    montar(s, |f| {
        f.titulo(6, "Referências", "Resistividades e seções comerciais; as notas normativas estão abaixo (linha 23).");
        f.cabecalho(3, 0, &["Material", "ρ a 20 °C (Ω·mm²/m)", "α (1/°C)"]);
        let dados = vec![linha(&["Cobre", "=1/56", "0,00393"]), linha(&["Alumínio", "=1/35,5", "0,00403"])];
        f.linhas(4, 0, &[Fmt::Texto, Fmt::Texto, Fmt::Texto], &dados);
        for r in 4..=5 {
            f.pintar(r, 1, r, 1, &num(5.0));
            f.pintar(r, 2, r, 2, &num(5.0));
        }
        f.cabecalho(3, 4, &["Seções comerciais (mm²)"]);
        let secoes: Vec<Vec<String>> = SECOES.iter().map(|x| vec![x.to_string()]).collect();
        f.linhas(4, 4, &[Fmt::Centro], &secoes);

        let notas = [
            ("Método", "ΔV = k · ρ · L · I · cos φ / S, com k = 2 (mono e bifásico) ou √3 (trifásico); I = P / (n · V · cos φ), n = √3 no trifásico e 1 nos demais. Sistema, fases e tensão (entre condutores energizados) valem por circuito."),
            ("Balanceamento", "Corrente: cada fase ligada ao circuito conduz a corrente de linha I (mono: 1 fase; bi: 2; tri: 3). Potência: mono entra inteira na fase, bi divide por dois e tri por três. Desequilíbrio = (máxima - média) / média; vale até o limite do projeto (padrão 10%, prática comum de concessionárias e NBR 5410, 4.2.5.6 — confira a edição vigente)."),
            ("Resistividade", "ρ(T) = ρ20 · [1 + α · (T - 20)]. A 70 °C o cobre fica em 0,0214 Ω·mm²/m. A planilha original usa 1/56 (cobre) e 1/35,5 (alumínio) a 20 °C."),
            ("Reatância", "Desprezada: vale bem para seções até 25 mm² ou FP alto. Em seções grandes e FP baixo, confira com R e X do fabricante."),
            ("Limites (NBR 5410, 6.2.7)", "Circuitos terminais: máx. 4%. Total desde o secundário do transformador MT/BT: até 7% (5% a partir do ponto de entrega, em outros casos). Confira a edição vigente."),
            ("Seção mínima", "É a menor seção comercial que cumpre o limite de queda. Não considera capacidade de condução de corrente, sobrecarga nem curto-circuito: a seção final é a maior entre os critérios."),
            ("Seções mínimas (NBR 5410, 6.2.6)", "Cobre: 1,5 mm² em iluminação e 2,5 mm² em força. Alumínio: 16 mm²."),
        ];
        for (i, (t, x)) in notas.iter().enumerate() {
            let r = 22 + i as u32;
            f.celula(r, 0, t, estilo().negrito().cor(ACO.forte).topo());
            f.celula(r, 1, x, estilo().cor(TEXTO).quebrar().topo());
            f.mesclar(r, 1, r, 6);
            f.altura(r, 46);
        }
        f.larguras(&[190, 175, 100, 28, 200, 100, 100]);
    });
}
