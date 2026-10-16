//! Ferramentas do código das macros (Alt+F11), 14/10/2026:
//! busca de macro pelo nome no Explorador, Localizar/Substituir (Ctrl+F /
//! Ctrl+H) dentro do código, desfazer/refazer do editor e a posição
//! "Ln X, Col Y". A parte de desenho fica em `editor_visual/codigo_view.rs`.
//!
//! O `text_editor` do Iced 0.13 não tem desfazer nem rolagem exposta. Por isso:
//! - o desfazer guarda retratos do texto (digitação seguida vira UMA ação;
//!   "Substituir todos" é sempre uma ação só);
//! - o editor fica dentro de uma rolagem nossa, com a altura exata do texto
//!   (linha de altura fixa), para a calha de números andar junto.
use super::*;
use iced::widget::text_editor::{Action, Content, Edit, Motion};
use iced::widget::{scrollable, text_input};

/// Altura fixa da linha do código (px). A calha de números usa a mesma.
pub(in crate::app) const ALTURA_LINHA: f32 = 18.0;
pub(in crate::app) const TAMANHO_FONTE: f32 = 13.0;
/// Margem interna do editor: vertical e horizontal.
pub(in crate::app) const MARGEM_V: f32 = 10.0;
pub(in crate::app) const MARGEM_H: f32 = 12.0;
/// Largura estimada de um caractere monoespaçado de 13 px (DejaVu Sans Mono
/// dá 7,8; sobra um pouco para não cortar a linha mais comprida).
pub(in crate::app) const LARGURA_CARACTERE: f32 = 8.0;
/// Teto de ocorrências contadas e destacadas.
const TETO_OCORRENCIAS: usize = 10_000;
/// Retratos guardados para desfazer.
const TETO_DESFAZER: usize = 200;

pub(in crate::app) fn id_rolagem() -> scrollable::Id { scrollable::Id::new("macro-codigo-vertical") }
pub(in crate::app) fn id_rolagem_h() -> scrollable::Id { scrollable::Id::new("macro-codigo-horizontal") }
pub(in crate::app) fn id_campo_busca() -> text_input::Id { text_input::Id::new("macro-busca-codigo") }
pub(in crate::app) fn id_campo_troca() -> text_input::Id { text_input::Id::new("macro-busca-troca") }
pub(in crate::app) fn id_campo_projeto() -> text_input::Id { text_input::Id::new("macro-busca-projeto") }
/// Contêiner do editor: o foco programático procura o editor dentro dele.
pub(in crate::app) const ID_AREA_CODIGO: &str = "macro-codigo-area";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum TipoEdicao { Digitar, Apagar, Outra }

impl TipoEdicao {
    fn de(edicao: &Edit) -> Self {
        match edicao {
            Edit::Insert(c) if !c.is_whitespace() => Self::Digitar,
            Edit::Backspace | Edit::Delete => Self::Apagar,
            _ => Self::Outra,
        }
    }
}

/// Texto e cursor (linha, coluna em caracteres) antes de uma edição.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) struct Retrato { pub texto: String, pub cursor: (usize, usize) }

#[derive(Debug, Clone)]
pub(in crate::app) struct EstadoCodigo {
    /// Busca do Explorador de projeto (nome do módulo ou da função).
    pub filtro_projeto: String,
    /// Barra de Localizar aberta; `substituir` mostra a segunda linha.
    pub busca_aberta: bool,
    pub substituir: bool,
    pub consulta: String,
    pub troca: String,
    pub maiusculas: bool,
    pub palavra: bool,
    pub todos_modulos: bool,
    /// Ocorrência atual (linha, coluna), ambas a partir de 0, em caracteres.
    pub atual: Option<(usize, usize)>,
    /// De onde a busca parte enquanto se digita a consulta.
    pub origem: (usize, usize),
    /// Shift pressionado (Shift+Enter no campo volta uma ocorrência).
    pub shift: bool,
    /// O foco está num campo da barra (Ctrl+Z não mexe no código).
    pub campo_focado: bool,
    /// Rolagem vertical e horizontal: (deslocamento, tamanho visível).
    pub rolagem_v: Option<(f32, f32)>,
    pub rolagem_h: Option<(f32, f32)>,
    /// Tamanho da janela do editor (largura mínima do código).
    pub janela: iced::Size,
    pub desfazer: Vec<Retrato>,
    pub refazer: Vec<Retrato>,
    /// Documento a que as pilhas de desfazer pertencem.
    pub dono: String,
    pub ultimo: Option<TipoEdicao>,
    /// Aviso curto da barra (ex.: "5 substituições").
    pub aviso: String,
}

impl Default for EstadoCodigo {
    fn default() -> Self {
        Self {
            filtro_projeto: String::new(), busca_aberta: false, substituir: false,
            consulta: String::new(), troca: String::new(), maiusculas: false,
            palavra: false, todos_modulos: false, atual: None, origem: (0, 0),
            shift: false, campo_focado: false, rolagem_v: None, rolagem_h: None,
            janela: iced::Size::new(1180.0, 760.0), desfazer: Vec::new(),
            refazer: Vec::new(), dono: String::new(), ultimo: None, aviso: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum EventoCodigo {
    FiltroProjeto(String),
    /// Enter na busca do Explorador: abre o primeiro resultado.
    FiltroAbrirPrimeiro,
    FocarProjeto,
    /// Abre o módulo e põe o cursor na linha (função achada pelo nome).
    IrParaFuncao(String, usize),
    /// Ctrl+F (false) ou Ctrl+H (true).
    Abrir(bool),
    Fechar,
    Consulta(String),
    Troca(String),
    Proximo,
    Anterior,
    /// Enter no campo de busca: Shift decide a direção.
    Enter,
    AlternarMaiusculas,
    AlternarPalavra,
    AlternarTodos,
    AlternarSubstituir,
    Substituir,
    SubstituirTodos,
    /// Resultado de "em todos os módulos": módulo, linha, coluna.
    IrPara(String, usize, usize),
    Desfazer,
    Refazer,
    Rolou(scrollable::Viewport),
    RolouH(scrollable::Viewport),
    Modificadores(keyboard::Modifiers),
}

// ---------------------------------------------------------------- funções puras

/// Funções declaradas num módulo: (nome, linha a partir de 0). Reconhece
/// `function f(`, `async function f(`, `const f = (…) =>`, `let f = function`.
pub(in crate::app) fn funcoes_do_modulo(codigo: &str) -> Vec<(String, usize)> {
    let mut saida = Vec::new();
    for (i, linha) in codigo.lines().enumerate() {
        let mut t = linha.trim_start();
        for prefixo in ["export ", "async "] {
            t = t.strip_prefix(prefixo).map(str::trim_start).unwrap_or(t);
        }
        let identificador = |s: &str| -> String {
            s.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$').collect()
        };
        if let Some(resto) = t.strip_prefix("function") {
            let nome = identificador(resto.trim_start_matches('*').trim_start());
            if !nome.is_empty() { saida.push((nome, i)); }
            continue;
        }
        for decl in ["const ", "let ", "var "] {
            let Some(resto) = t.strip_prefix(decl) else { continue };
            let nome = identificador(resto.trim_start());
            let depois = resto.trim_start()[nome.len()..].trim_start();
            let Some(valor) = depois.strip_prefix('=').filter(|v| !v.starts_with('=')) else { break };
            let valor = valor.trim_start();
            if !nome.is_empty() && (valor.starts_with("function") || valor.starts_with("async")
                || valor.contains("=>")) {
                saida.push((nome, i));
            }
            break;
        }
    }
    saida
}

/// Um módulo na busca do Explorador, com as funções que bateram.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) struct ItemFiltro {
    pub modulo: String,
    pub auto: bool,
    pub nome_bate: bool,
    pub funcoes: Vec<(String, usize)>,
}

/// Filtra os módulos pelo nome do módulo ou de uma função dele, sem
/// diferenciar acento nem maiúscula (`feather_ui::ajuda::normalizar`).
pub(in crate::app) fn filtrar_modulos(macros: &[feather_macros::Macro], filtro: &str) -> Vec<ItemFiltro> {
    let alvo = feather_ui::ajuda::normalizar(filtro.trim());
    macros.iter().filter_map(|m| {
        if alvo.is_empty() {
            return Some(ItemFiltro { modulo: m.name.clone(), auto: m.auto, nome_bate: true, funcoes: Vec::new() });
        }
        let nome_bate = feather_ui::ajuda::normalizar(&m.name).contains(&alvo);
        let funcoes: Vec<_> = funcoes_do_modulo(&m.code).into_iter()
            .filter(|(f, _)| feather_ui::ajuda::normalizar(f).contains(&alvo)).collect();
        (nome_bate || !funcoes.is_empty()).then(|| ItemFiltro {
            modulo: m.name.clone(), auto: m.auto, nome_bate, funcoes,
        })
    }).collect()
}

/// Uma ocorrência no código: linha e coluna a partir de 0, tamanho em caracteres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::app) struct Ocorrencia { pub linha: usize, pub coluna: usize, pub tamanho: usize }

fn parte_de_palavra(c: char) -> bool { c.is_alphanumeric() || c == '_' || c == '$' }

/// Todas as ocorrências (sem sobreposição), linha a linha.
pub(in crate::app) fn ocorrencias(texto: &str, consulta: &str, maiusculas: bool, palavra: bool) -> Vec<Ocorrencia> {
    let q: Vec<char> = consulta.chars().collect();
    if q.is_empty() || q.contains(&'\n') { return Vec::new(); }
    let igual = |a: char, b: char| a == b || (!maiusculas && a.to_lowercase().eq(b.to_lowercase()));
    let mut saida = Vec::new();
    for (l, linha) in texto.split('\n').enumerate() {
        let cs: Vec<char> = linha.chars().collect();
        let mut i = 0;
        while i + q.len() <= cs.len() {
            let bate = cs[i..i + q.len()].iter().zip(&q).all(|(a, b)| igual(*a, *b))
                && (!palavra || (i.checked_sub(1).is_none_or(|p| !parte_de_palavra(cs[p]))
                    && cs.get(i + q.len()).is_none_or(|c| !parte_de_palavra(*c))));
            if bate {
                saida.push(Ocorrencia { linha: l, coluna: i, tamanho: q.len() });
                if saida.len() >= TETO_OCORRENCIAS { return saida; }
                i += q.len();
            } else { i += 1; }
        }
    }
    saida
}

/// Troca todas as ocorrências; devolve o texto novo e quantas trocou.
pub(in crate::app) fn substituir_todas(texto: &str, achadas: &[Ocorrencia], troca: &str) -> (String, usize) {
    let mut linhas: Vec<String> = texto.split('\n').map(str::to_owned).collect();
    // De trás para frente: a troca não desloca as colunas que faltam.
    for o in achadas.iter().rev() {
        let Some(linha) = linhas.get_mut(o.linha) else { continue };
        let cs: Vec<char> = linha.chars().collect();
        if o.coluna + o.tamanho > cs.len() { continue; }
        let novo: String = cs[..o.coluna].iter().collect::<String>() + troca
            + &cs[o.coluna + o.tamanho..].iter().collect::<String>();
        *linha = novo;
    }
    (linhas.join("\n"), achadas.len())
}

/// Texto exato do editor, linha a linha. O `Content::text()` do Iced 0.13
/// sempre acrescenta "\n" no fim, e assim "abc" e "abc" + linha vazia saem
/// iguais; o desfazer precisa distinguir os dois.
pub(in crate::app) fn texto_exato(conteudo: &Content) -> String {
    conteudo.lines().map(|l| l.to_string()).collect::<Vec<_>>().join("\n")
}

/// Recria o conteúdo a partir do [`texto_exato`]. O `with_text` descarta um
/// "\n" final, então ele vai acrescentado.
pub(in crate::app) fn conteudo_de(texto: &str) -> Content {
    Content::with_text(&format!("{texto}\n"))
}

/// Linha e coluna do cursor, contadas a partir de 1, em caracteres.
pub(in crate::app) fn linha_coluna(conteudo: &Content) -> (usize, usize) {
    let (linha, indice) = conteudo.cursor_position();
    let coluna = conteudo.line(linha)
        .map(|t| t.get(..indice.min(t.len())).map_or(0, |s| s.chars().count()))
        .unwrap_or(0);
    (linha + 1, coluna + 1)
}

/// Põe o cursor na linha/coluna (a partir de 0, em caracteres).
pub(in crate::app) fn posicionar(conteudo: &mut Content, linha: usize, coluna: usize) {
    conteudo.perform(Action::Move(Motion::DocumentStart));
    let linha = linha.min(conteudo.line_count().saturating_sub(1));
    for _ in 0..linha { conteudo.perform(Action::Move(Motion::Down)); }
    conteudo.perform(Action::Move(Motion::Home));
    let tamanho = conteudo.line(linha).map_or(0, |t| t.chars().count());
    for _ in 0..coluna.min(tamanho) { conteudo.perform(Action::Move(Motion::Right)); }
}

/// Seleciona `tamanho` caracteres a partir da posição.
pub(in crate::app) fn selecionar(conteudo: &mut Content, o: Ocorrencia) {
    posicionar(conteudo, o.linha, o.coluna);
    for _ in 0..o.tamanho { conteudo.perform(Action::Select(Motion::Right)); }
}

/// Texto da seleção, se for de uma linha só (preenche a busca ao abrir).
fn selecao_de_uma_linha(conteudo: &Content) -> Option<String> {
    conteudo.selection().filter(|s| !s.is_empty() && !s.contains('\n'))
}

// ---------------------------------------------------------------- foco no editor

/// Foca o primeiro campo focável dentro do contêiner [`ID_AREA_CODIGO`] (o
/// `text_editor` não tem Id próprio no Iced 0.13) e tira o foco dos demais.
struct FocarCodigo { dentro: bool, feito: bool }

impl iced::advanced::widget::Operation<Msg> for FocarCodigo {
    fn container(&mut self, id: Option<&iced::advanced::widget::Id>, _limites: iced::Rectangle,
        filhos: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation<Msg>)) {
        let antes = self.dentro;
        if id == Some(&iced::advanced::widget::Id::new(ID_AREA_CODIGO)) { self.dentro = true; }
        filhos(self);
        self.dentro = antes;
    }
    fn focusable(&mut self, estado: &mut dyn iced::advanced::widget::operation::Focusable,
        _id: Option<&iced::advanced::widget::Id>) {
        if self.dentro && !self.feito { estado.focus(); self.feito = true; } else { estado.unfocus(); }
    }
}

fn focar_codigo() -> Task<Msg> {
    iced::advanced::widget::operate(FocarCodigo { dentro: false, feito: false })
}

// ---------------------------------------------------------------- App

impl App {
    /// Ocorrências da consulta no código aberto.
    pub(in crate::app) fn ocorrencias_atuais(&self) -> Vec<Ocorrencia> {
        let c = &self.macro_visual.codigo;
        if !c.busca_aberta { return Vec::new(); }
        ocorrencias(&texto_exato(&self.macro_codigo), &c.consulta, c.maiusculas, c.palavra)
    }

    /// Resultados em todos os módulos: (módulo, ocorrência, trecho da linha).
    /// O módulo aberto usa o texto do editor, não o guardado.
    pub(in crate::app) fn ocorrencias_em_todos(&self) -> Vec<(String, Ocorrencia, String)> {
        let c = &self.macro_visual.codigo;
        let mut saida = Vec::new();
        let mut modulos: Vec<(String, String)> = self.macro_catalogo.macros.iter()
            .map(|m| (m.name.clone(), m.code.clone())).collect();
        match modulos.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(&self.macro_nome)) {
            Some(m) => m.1 = self.macro_codigo.text(),
            None => modulos.insert(0, (self.macro_nome.clone(), self.macro_codigo.text())),
        }
        for (nome, codigo) in modulos {
            let linhas: Vec<&str> = codigo.split('\n').collect();
            for o in ocorrencias(&codigo, &c.consulta, c.maiusculas, c.palavra) {
                let trecho = linhas.get(o.linha).map_or(String::new(), |l| l.trim().chars().take(80).collect());
                saida.push((nome.clone(), o, trecho));
                if saida.len() >= 500 { return saida; }
            }
        }
        saida
    }

    /// Esvazia o desfazer quando outro documento entra no editor.
    fn sincronizar_dono(&mut self) {
        let c = &mut self.macro_visual.codigo;
        if c.dono != self.macro_visual.documento {
            c.dono = self.macro_visual.documento.clone();
            c.desfazer.clear();
            c.refazer.clear();
            c.ultimo = None;
        }
    }

    fn retrato(&self) -> Retrato {
        let (l, c) = linha_coluna(&self.macro_codigo);
        Retrato { texto: texto_exato(&self.macro_codigo), cursor: (l - 1, c - 1) }
    }

    /// Guarda o estado antes de uma edição; digitação seguida é uma ação só.
    fn anotar_desfazer(&mut self, tipo: TipoEdicao) {
        self.sincronizar_dono();
        let agrupa = tipo != TipoEdicao::Outra && self.macro_visual.codigo.ultimo == Some(tipo);
        if !agrupa {
            let r = self.retrato();
            let c = &mut self.macro_visual.codigo;
            c.desfazer.push(r);
            if c.desfazer.len() > TETO_DESFAZER { c.desfazer.remove(0); }
        }
        let c = &mut self.macro_visual.codigo;
        c.refazer.clear();
        c.ultimo = Some(tipo);
    }

    fn restaurar(&mut self, r: Retrato) {
        self.macro_codigo = conteudo_de(&r.texto);
        posicionar(&mut self.macro_codigo, r.cursor.0, r.cursor.1);
        self.macro_visual.codigo.ultimo = None;
    }

    pub(in crate::app) fn desfazer_codigo(&mut self) -> bool {
        self.sincronizar_dono();
        let Some(r) = self.macro_visual.codigo.desfazer.pop() else { return false };
        let agora = self.retrato();
        self.macro_visual.codigo.refazer.push(agora);
        self.restaurar(r);
        true
    }

    pub(in crate::app) fn refazer_codigo(&mut self) -> bool {
        self.sincronizar_dono();
        let Some(r) = self.macro_visual.codigo.refazer.pop() else { return false };
        let agora = self.retrato();
        self.macro_visual.codigo.desfazer.push(agora);
        self.restaurar(r);
        true
    }

    /// Ação do `text_editor`: anota o desfazer, aplica e rola até o cursor.
    pub(in crate::app) fn editar_codigo_macro(&mut self, acao: Action) -> Task<Msg> {
        if let Action::Scroll { lines } = acao {
            // O editor tem a altura do texto; quem rola é a nossa rolagem.
            return scrollable::scroll_by(id_rolagem(), scrollable::AbsoluteOffset {
                x: 0.0, y: lines as f32 * ALTURA_LINHA,
            });
        }
        if matches!(acao, Action::Click(_) | Action::Drag(_)) {
            self.macro_visual.codigo.campo_focado = false;
        }
        match &acao {
            Action::Edit(edicao) => {
                let tipo = TipoEdicao::de(edicao);
                self.anotar_desfazer(tipo);
            }
            _ => self.macro_visual.codigo.ultimo = None,
        }
        let rolar = !matches!(acao, Action::Click(_) | Action::Drag(_));
        self.macro_codigo.perform(acao);
        if rolar { self.rolar_ate_cursor() } else { Task::none() }
    }

    /// Rola o código até a linha/coluna do cursor, se estiver fora da vista.
    pub(in crate::app) fn rolar_ate_cursor(&self) -> Task<Msg> {
        let (l, c) = linha_coluna(&self.macro_codigo);
        self.rolar_ate(l - 1, c - 1)
    }

    fn rolar_ate(&self, linha: usize, coluna: usize) -> Task<Msg> {
        let estado = &self.macro_visual.codigo;
        let mut tarefas = Vec::new();
        let topo = MARGEM_V + linha as f32 * ALTURA_LINHA;
        let (desl, altura) = estado.rolagem_v.unwrap_or((0.0, f32::INFINITY));
        let alvo_y = if topo < desl {
            Some((topo - ALTURA_LINHA).max(0.0))
        } else if topo + ALTURA_LINHA > desl + altura {
            Some(topo + 2.0 * ALTURA_LINHA - altura)
        } else { None };
        if let Some(y) = alvo_y {
            tarefas.push(scrollable::scroll_to(id_rolagem(), scrollable::AbsoluteOffset { x: 0.0, y }));
        }
        let x = MARGEM_H + coluna as f32 * LARGURA_CARACTERE;
        if let Some((desl_x, largura)) = estado.rolagem_h {
            let alvo_x = if x < desl_x + MARGEM_H {
                Some((x - 4.0 * LARGURA_CARACTERE).max(0.0))
            } else if x + 2.0 * LARGURA_CARACTERE > desl_x + largura {
                Some(x + 6.0 * LARGURA_CARACTERE - largura)
            } else { None };
            if let Some(x) = alvo_x {
                tarefas.push(scrollable::scroll_to(id_rolagem_h(), scrollable::AbsoluteOffset { x, y: 0.0 }));
            }
        }
        Task::batch(tarefas)
    }

    /// Torna `o` a ocorrência atual: seleciona no editor e rola até ela.
    fn ir_para_ocorrencia(&mut self, o: Ocorrencia) -> Task<Msg> {
        self.macro_visual.codigo.atual = Some((o.linha, o.coluna));
        selecionar(&mut self.macro_codigo, o);
        self.macro_visual.codigo.ultimo = None;
        self.rolar_ate(o.linha, o.coluna)
    }

    /// Próxima (ou anterior) ocorrência a partir da atual ou do cursor; dá a volta.
    fn andar_busca(&mut self, para_frente: bool) -> Task<Msg> {
        let achadas = self.ocorrencias_atuais();
        if achadas.is_empty() { return Task::none(); }
        let referencia = self.macro_visual.codigo.atual.unwrap_or_else(|| {
            let (l, c) = linha_coluna(&self.macro_codigo);
            (l - 1, c - 1)
        });
        let atual_valida = self.macro_visual.codigo.atual.is_some();
        let pos = |o: &Ocorrencia| (o.linha, o.coluna);
        let escolhida = if para_frente {
            achadas.iter().find(|o| if atual_valida { pos(o) > referencia } else { pos(o) >= referencia })
                .or(achadas.first())
        } else {
            achadas.iter().rev().find(|o| pos(o) < referencia).or(achadas.last())
        };
        match escolhida.copied() { Some(o) => self.ir_para_ocorrencia(o), None => Task::none() }
    }

    /// Recalcula a ocorrência atual depois de mudar a consulta ou as opções.
    fn reprocurar(&mut self) -> Task<Msg> {
        let origem = self.macro_visual.codigo.origem;
        let achadas = self.ocorrencias_atuais();
        let escolhida = achadas.iter().find(|o| (o.linha, o.coluna) >= origem).or(achadas.first()).copied();
        self.macro_visual.codigo.aviso.clear();
        match escolhida {
            Some(o) => self.ir_para_ocorrencia(o),
            None => { self.macro_visual.codigo.atual = None; Task::none() }
        }
    }

    pub(in crate::app) fn abrir_busca_codigo(&mut self, substituir: bool) -> Task<Msg> {
        if !self.macro_visual.documento.starts_with("modulo:") {
            self.macro_saida = "Abra um módulo JavaScript para localizar no código.".into();
            return Task::none();
        }
        let selecionado = selecao_de_uma_linha(&self.macro_codigo);
        let (l, c) = linha_coluna(&self.macro_codigo);
        let estado = &mut self.macro_visual.codigo;
        estado.busca_aberta = true;
        estado.substituir = substituir;
        estado.campo_focado = true;
        estado.aviso.clear();
        if let Some(s) = selecionado {
            // A seleção começa antes do cursor: procurar a partir do início dela.
            estado.origem = (l - 1, (c - 1).saturating_sub(s.chars().count()));
            estado.consulta = s;
        } else {
            estado.origem = (l - 1, c - 1);
        }
        estado.atual = None;
        let foco = if substituir && !self.macro_visual.codigo.consulta.is_empty() {
            text_input::focus(id_campo_troca())
        } else { text_input::focus(id_campo_busca()) };
        let selecionar_tudo = text_input::select_all(id_campo_busca());
        let busca = self.reprocurar();
        Task::batch([foco, selecionar_tudo, busca])
    }

    fn substituir_atual_codigo(&mut self) -> Task<Msg> {
        let achadas = self.ocorrencias_atuais();
        let atual = self.macro_visual.codigo.atual
            .and_then(|p| achadas.iter().find(|o| (o.linha, o.coluna) == p).copied());
        let Some(o) = atual else { return self.andar_busca(true) };
        self.anotar_desfazer(TipoEdicao::Outra);
        selecionar(&mut self.macro_codigo, o);
        let troca = self.macro_visual.codigo.troca.clone();
        let tamanho = troca.chars().count();
        if troca.is_empty() {
            self.macro_codigo.perform(Action::Edit(Edit::Delete));
        } else {
            self.macro_codigo.perform(Action::Edit(Edit::Paste(std::sync::Arc::new(troca))));
        }
        self.macro_visual.codigo.ultimo = None;
        // Segue para a próxima depois do texto trocado (a troca pode conter a
        // consulta: não procurar dentro dela).
        self.macro_visual.codigo.origem = (o.linha, o.coluna + tamanho);
        self.reprocurar()
    }

    fn substituir_todas_codigo(&mut self) -> Task<Msg> {
        let achadas = self.ocorrencias_atuais();
        if achadas.is_empty() {
            self.macro_visual.codigo.aviso = "Nenhuma ocorrência".into();
            return Task::none();
        }
        let (l, c) = linha_coluna(&self.macro_codigo);
        let (novo, n) = substituir_todas(&texto_exato(&self.macro_codigo), &achadas, &self.macro_visual.codigo.troca);
        // Uma ação só no desfazer, por maior que seja a troca.
        self.anotar_desfazer(TipoEdicao::Outra);
        self.macro_codigo = conteudo_de(&novo);
        posicionar(&mut self.macro_codigo, l - 1, c - 1);
        let estado = &mut self.macro_visual.codigo;
        estado.ultimo = None;
        estado.atual = None;
        estado.aviso = if n == 1 { "1 substituição".into() } else { format!("{n} substituições") };
        Task::none()
    }

    fn abrir_modulo_na_linha(&mut self, modulo: &str, linha: usize, coluna: usize, tamanho: usize) -> Task<Msg> {
        if !self.macro_nome.eq_ignore_ascii_case(modulo) || !self.macro_visual.documento.starts_with("modulo:") {
            self.evento_macro_visual(EventoVisual::Documento(format!("modulo:{modulo}")));
        }
        if !self.macro_nome.eq_ignore_ascii_case(modulo) { return Task::none(); }
        let o = Ocorrencia { linha, coluna, tamanho };
        if tamanho == 0 {
            posicionar(&mut self.macro_codigo, linha, coluna);
            self.macro_visual.codigo.campo_focado = false;
            return Task::batch([self.rolar_ate(linha, coluna), focar_codigo()]);
        }
        self.ir_para_ocorrencia(o)
    }

    /// Esc: fecha a barra de busca ou limpa a busca do Explorador.
    /// Devolve `None` quando não havia nada para fechar.
    pub(in crate::app) fn escape_codigo(&mut self) -> Option<Task<Msg>> {
        if self.macro_visual.codigo.busca_aberta {
            return Some(self.evento_codigo_macro(EventoCodigo::Fechar));
        }
        if !self.macro_visual.codigo.filtro_projeto.is_empty() {
            self.macro_visual.codigo.filtro_projeto.clear();
            return Some(Task::none());
        }
        None
    }

    /// `FEATHERSHEET_CAPTURAR_JANELA=busca_macros`: abre o editor com um
    /// módulo de exemplo (só na memória, nada é gravado), a busca do
    /// Explorador com "total", Ctrl+H com "sheet" → "planilha" e o cursor na
    /// linha 9, para fotografar a barra, os destaques e a calha de números.
    pub(in crate::app) fn preparar_captura_busca_macros(&mut self) -> Task<Msg> {
        let exemplo = [
            "// Relatório mensal · JavaScript do FeatherSheet",
            "function calcularTotal(coluna) {",
            "  let total = 0;",
            "  for (let linha = 2; linha <= sheet.lastRow(); linha++) {",
            "    total += Number(sheet.value(coluna + linha)) || 0;",
            "  }",
            "  return total;",
            "}",
            "",
            "const limparResumo = () => sheet.clear('E1:E10');",
            "",
            "limparResumo();",
            "sheet.set('E1', 'Total de vendas');",
            "sheet.set('E2', calcularTotal('B'));",
            "sheet.style('E1:E2', { bold: true });",
            "log('Resumo atualizado: ' + sheet.value('E2'));",
        ].join("\n");
        self.macro_catalogo = feather_macros::Catalogo {
            macros: vec![
                feather_macros::Macro { name: "Relatorio".into(), code: exemplo.clone(), auto: false },
                feather_macros::Macro { name: "Importar".into(), code: "function totalPorMes() {}\n".into(), auto: false },
                feather_macros::Macro { name: "Limpar".into(), code: "sheet.clear('A1:D10');\n".into(), auto: true },
            ],
        };
        self.macro_nome = "Relatorio".into();
        self.macro_visual.documento = "modulo:Relatorio".into();
        self.macro_codigo = Content::with_text(&exemplo);
        self.macro_visual.codigo.filtro_projeto = "TOTAL".into();
        posicionar(&mut self.macro_codigo, 8, 0);
        let estado = &mut self.macro_visual.codigo;
        // FEATHERSHEET_CAPTURAR_BUSCA troca o termo procurado ("-" deixa a barra
        // fechada; "*" na frente liga "Todos os módulos").
        let termo = std::env::var("FEATHERSHEET_CAPTURAR_BUSCA").unwrap_or_else(|_| "sheet".into());
        let (termo, todos) = match termo.strip_prefix('*') { Some(t) => (t.to_owned(), true), None => (termo, false) };
        estado.todos_modulos = todos;
        estado.busca_aberta = termo != "-";
        estado.substituir = true;
        estado.consulta = if termo == "-" { String::new() } else { termo };
        estado.troca = "planilha".into();
        estado.origem = (8, 0);
        let _ = self.reprocurar();
        self.macro_visual.console = false;
        self.alternar(Auxiliar::Macros)
    }

    pub(in crate::app) fn evento_codigo_macro(&mut self, evento: EventoCodigo) -> Task<Msg> {
        match evento {
            EventoCodigo::FiltroProjeto(t) => { self.macro_visual.codigo.filtro_projeto = t; Task::none() }
            EventoCodigo::FocarProjeto => text_input::focus(id_campo_projeto()),
            EventoCodigo::FiltroAbrirPrimeiro => {
                let itens = filtrar_modulos(&self.macro_catalogo.macros, &self.macro_visual.codigo.filtro_projeto);
                let Some(primeiro) = itens.first() else { return Task::none() };
                let linha = if primeiro.nome_bate { None } else { primeiro.funcoes.first().map(|f| f.1) };
                let modulo = primeiro.modulo.clone();
                match linha {
                    Some(l) => self.abrir_modulo_na_linha(&modulo, l, 0, 0),
                    None => {
                        self.evento_macro_visual(EventoVisual::Documento(format!("modulo:{modulo}")));
                        focar_codigo()
                    }
                }
            }
            EventoCodigo::IrParaFuncao(modulo, linha) => self.abrir_modulo_na_linha(&modulo, linha, 0, 0),
            EventoCodigo::Abrir(substituir) => self.abrir_busca_codigo(substituir),
            EventoCodigo::Fechar => {
                let estado = &mut self.macro_visual.codigo;
                estado.busca_aberta = false;
                estado.campo_focado = false;
                estado.aviso.clear();
                // Volta o foco ao código, com a ocorrência atual selecionada.
                focar_codigo()
            }
            EventoCodigo::Consulta(t) => {
                self.macro_visual.codigo.consulta = t;
                self.macro_visual.codigo.campo_focado = true;
                self.reprocurar()
            }
            EventoCodigo::Troca(t) => {
                self.macro_visual.codigo.troca = t;
                self.macro_visual.codigo.campo_focado = true;
                Task::none()
            }
            EventoCodigo::Proximo => self.andar_busca(true),
            EventoCodigo::Anterior => self.andar_busca(false),
            EventoCodigo::Enter => {
                let frente = !self.macro_visual.codigo.shift;
                self.andar_busca(frente)
            }
            EventoCodigo::AlternarMaiusculas => {
                self.macro_visual.codigo.maiusculas ^= true;
                self.reprocurar()
            }
            EventoCodigo::AlternarPalavra => {
                self.macro_visual.codigo.palavra ^= true;
                self.reprocurar()
            }
            EventoCodigo::AlternarTodos => { self.macro_visual.codigo.todos_modulos ^= true; Task::none() }
            EventoCodigo::AlternarSubstituir => {
                self.macro_visual.codigo.substituir ^= true;
                if self.macro_visual.codigo.substituir { text_input::focus(id_campo_troca()) } else { Task::none() }
            }
            EventoCodigo::Substituir => self.substituir_atual_codigo(),
            EventoCodigo::SubstituirTodos => self.substituir_todas_codigo(),
            EventoCodigo::IrPara(modulo, linha, coluna) => {
                let tamanho = self.macro_visual.codigo.consulta.chars().count();
                self.abrir_modulo_na_linha(&modulo, linha, coluna, tamanho)
            }
            EventoCodigo::Desfazer => {
                if self.desfazer_codigo() { self.rolar_ate_cursor() } else { Task::none() }
            }
            EventoCodigo::Refazer => {
                if self.refazer_codigo() { self.rolar_ate_cursor() } else { Task::none() }
            }
            EventoCodigo::Rolou(v) => {
                let o = v.absolute_offset();
                let primeira = self.macro_visual.codigo.rolagem_v.is_none();
                self.macro_visual.codigo.rolagem_v = Some((o.y, v.bounds().height));
                // Só agora se sabe a altura visível: leva a ocorrência (ou o
                // cursor) para a vista se o editor abriu já com ela lá embaixo.
                match self.macro_visual.codigo.atual.filter(|_| primeira) {
                    Some((l, c)) => self.rolar_ate(l, c),
                    None if primeira => self.rolar_ate_cursor(),
                    None => Task::none(),
                }
            }
            EventoCodigo::RolouH(v) => {
                let o = v.absolute_offset();
                self.macro_visual.codigo.rolagem_h = Some((o.x, v.bounds().width));
                Task::none()
            }
            EventoCodigo::Modificadores(m) => { self.macro_visual.codigo.shift = m.shift(); Task::none() }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn modulo(nome: &str, codigo: &str) -> feather_macros::Macro {
        feather_macros::Macro { name: nome.into(), code: codigo.into(), auto: false }
    }

    /// O `Content::text()` do Iced sempre termina em "\n"; compara sem ele.
    fn texto(app: &App) -> String {
        texto_exato(&app.macro_codigo)
    }

    fn app_com_codigo(codigo: &str) -> App {
        let mut app = App::default();
        app.macro_catalogo = Default::default();
        app.macro_nome = "Macro1".into();
        app.macro_visual.documento = "modulo:Macro1".into();
        app.macro_codigo = Content::with_text(codigo);
        app
    }

    #[test]
    fn busca_do_projeto_ignora_acento_e_maiuscula_e_acha_funcao() {
        let macros = vec![
            modulo("Relatório", "log(1);"),
            modulo("Importar", "function calcularTotal() {}\nconst limparAbas = () => 1;\nlet x = 3;"),
            modulo("Outro", "async function ÚltimaLinha() {}"),
        ];
        let r = filtrar_modulos(&macros, "RELATORIO");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].modulo, "Relatório");
        assert!(r[0].nome_bate);
        let r = filtrar_modulos(&macros, "limpar");
        assert_eq!(r[0].modulo, "Importar");
        assert!(!r[0].nome_bate);
        assert_eq!(r[0].funcoes, vec![("limparAbas".to_string(), 1)]);
        let r = filtrar_modulos(&macros, "ultima");
        assert_eq!(r[0].funcoes, vec![("ÚltimaLinha".to_string(), 0)]);
        assert_eq!(filtrar_modulos(&macros, "").len(), 3);
        assert!(filtrar_modulos(&macros, "x").iter().all(|i| i.funcoes.is_empty() || i.modulo != "Importar"));
        assert!(filtrar_modulos(&macros, "nada-disso").is_empty());
    }

    #[test]
    fn enter_na_busca_do_projeto_abre_a_primeira_e_esc_limpa() {
        let mut app = app_com_codigo("log(0);");
        app.macro_catalogo.macros = vec![
            modulo("Macro1", "log(0);"),
            modulo("Contas", "log(1);\n\nfunction somarTudo() {}\n"),
        ];
        let _ = app.evento_codigo_macro(EventoCodigo::FiltroProjeto("SOMAR".into()));
        let _ = app.evento_codigo_macro(EventoCodigo::FiltroAbrirPrimeiro);
        assert_eq!(app.macro_visual.documento, "modulo:Contas");
        assert_eq!(linha_coluna(&app.macro_codigo), (3, 1));
        assert!(app.escape_codigo().is_some());
        assert!(app.macro_visual.codigo.filtro_projeto.is_empty());
        assert!(app.escape_codigo().is_none());
    }

    #[test]
    fn ocorrencias_respeitam_maiusculas_e_palavra_inteira() {
        let t = "Total = total + subtotal;\nTOTAL";
        assert_eq!(ocorrencias(t, "total", false, false).len(), 4);
        assert_eq!(ocorrencias(t, "total", true, false).len(), 2);
        let palavra = ocorrencias(t, "total", false, true);
        assert_eq!(palavra, vec![
            Ocorrencia { linha: 0, coluna: 0, tamanho: 5 },
            Ocorrencia { linha: 0, coluna: 8, tamanho: 5 },
            Ocorrencia { linha: 1, coluna: 0, tamanho: 5 },
        ]);
        assert!(ocorrencias(t, "", false, false).is_empty());
        // Acento conta como caractere (coluna em caracteres, não em bytes).
        assert_eq!(ocorrencias("ação ação", "ão", false, false)[1].coluna, 7);
    }

    #[test]
    fn localizar_proximo_e_anterior_com_contagem_e_volta() {
        let mut app = app_com_codigo("a = 1;\nb = a + a;\nlog(a);");
        let _ = app.evento_codigo_macro(EventoCodigo::Abrir(false));
        let _ = app.evento_codigo_macro(EventoCodigo::Consulta("a".into()));
        let achadas = app.ocorrencias_atuais();
        assert_eq!(achadas.len(), 4);
        let indice = |app: &App| {
            let p = app.macro_visual.codigo.atual.unwrap();
            app.ocorrencias_atuais().iter().position(|o| (o.linha, o.coluna) == p).unwrap() + 1
        };
        assert_eq!(indice(&app), 1);
        assert_eq!(app.macro_codigo.selection().as_deref(), Some("a"));
        let _ = app.evento_codigo_macro(EventoCodigo::Proximo);
        assert_eq!(indice(&app), 2);
        assert_eq!(app.macro_visual.codigo.atual, Some((1, 4)));
        assert_eq!(linha_coluna(&app.macro_codigo), (2, 6));
        let _ = app.evento_codigo_macro(EventoCodigo::Proximo);
        let _ = app.evento_codigo_macro(EventoCodigo::Proximo);
        assert_eq!(indice(&app), 4);
        let _ = app.evento_codigo_macro(EventoCodigo::Proximo);
        assert_eq!(indice(&app), 1, "dá a volta para a primeira");
        let _ = app.evento_codigo_macro(EventoCodigo::Anterior);
        assert_eq!(indice(&app), 4, "e para trás também");
        // Shift+Enter volta; Enter avança.
        let _ = app.evento_codigo_macro(EventoCodigo::Modificadores(keyboard::Modifiers::SHIFT));
        let _ = app.evento_codigo_macro(EventoCodigo::Enter);
        assert_eq!(indice(&app), 3);
        let _ = app.evento_codigo_macro(EventoCodigo::Modificadores(keyboard::Modifiers::default()));
        let _ = app.evento_codigo_macro(EventoCodigo::Enter);
        assert_eq!(indice(&app), 4);
        let _ = app.evento_codigo_macro(EventoCodigo::Fechar);
        assert!(!app.macro_visual.codigo.busca_aberta);
    }

    #[test]
    fn abrir_busca_preenche_com_a_selecao() {
        let mut app = app_com_codigo("let valor = 1;\nlog(valor);");
        posicionar(&mut app.macro_codigo, 0, 4);
        for _ in 0..5 { app.macro_codigo.perform(Action::Select(Motion::Right)); }
        let _ = app.evento_codigo_macro(EventoCodigo::Abrir(true));
        assert_eq!(app.macro_visual.codigo.consulta, "valor");
        assert!(app.macro_visual.codigo.substituir);
        assert_eq!(app.macro_visual.codigo.atual, Some((0, 4)));
    }

    #[test]
    fn substituir_todos_e_um_unico_desfazer() {
        let original = "x = 1;\nx = x + 2;\nlog(x);";
        let mut app = app_com_codigo(original);
        let _ = app.evento_codigo_macro(EventoCodigo::Abrir(true));
        let _ = app.evento_codigo_macro(EventoCodigo::Consulta("x".into()));
        let _ = app.evento_codigo_macro(EventoCodigo::Troca("total".into()));
        let _ = app.evento_codigo_macro(EventoCodigo::SubstituirTodos);
        assert_eq!(texto(&app), "total = 1;\ntotal = total + 2;\nlog(total);");
        assert_eq!(app.macro_visual.codigo.aviso, "4 substituições");
        assert_eq!(app.macro_visual.codigo.desfazer.len(), 1);
        let _ = app.evento_codigo_macro(EventoCodigo::Desfazer);
        assert_eq!(texto(&app), original);
        let _ = app.evento_codigo_macro(EventoCodigo::Refazer);
        assert!(texto(&app).starts_with("total = 1;"));
    }

    #[test]
    fn substituir_um_troca_a_atual_e_segue() {
        let mut app = app_com_codigo("a a a");
        let _ = app.evento_codigo_macro(EventoCodigo::Abrir(true));
        let _ = app.evento_codigo_macro(EventoCodigo::Consulta("a".into()));
        let _ = app.evento_codigo_macro(EventoCodigo::Troca("bb".into()));
        let _ = app.evento_codigo_macro(EventoCodigo::Substituir);
        assert_eq!(texto(&app), "bb a a");
        assert_eq!(app.macro_visual.codigo.atual, Some((0, 3)));
        let _ = app.evento_codigo_macro(EventoCodigo::Substituir);
        assert_eq!(texto(&app), "bb bb a");
        let _ = app.evento_codigo_macro(EventoCodigo::Desfazer);
        assert_eq!(texto(&app), "bb a a");
    }

    #[test]
    fn digitacao_seguida_vira_uma_acao_no_desfazer() {
        let mut app = app_com_codigo("");
        for c in "abc".chars() { let _ = app.editar_codigo_macro(Action::Edit(Edit::Insert(c))); }
        let _ = app.editar_codigo_macro(Action::Edit(Edit::Enter));
        for c in "de".chars() { let _ = app.editar_codigo_macro(Action::Edit(Edit::Insert(c))); }
        assert_eq!(texto(&app), "abc\nde");
        let _ = app.evento_codigo_macro(EventoCodigo::Desfazer);
        assert_eq!(texto(&app), "abc\n");
        let _ = app.evento_codigo_macro(EventoCodigo::Desfazer);
        assert_eq!(texto(&app), "abc");
        let _ = app.evento_codigo_macro(EventoCodigo::Desfazer);
        assert_eq!(texto(&app), "");
        let _ = app.evento_codigo_macro(EventoCodigo::Refazer);
        assert_eq!(texto(&app), "abc");
        // Outro documento: o desfazer não atravessa de um módulo para outro.
        app.macro_visual.documento = "modulo:Outro".into();
        assert!(!app.desfazer_codigo());
    }

    #[test]
    fn texto_exato_volta_igual_com_e_sem_linha_vazia_no_fim() {
        for t in ["", "abc", "abc\n", "abc\n\n", "a\nb"] {
            assert_eq!(texto_exato(&conteudo_de(t)), t);
        }
    }

    #[test]
    fn linha_e_coluna_contam_caracteres_a_partir_de_1() {
        let mut c = Content::with_text("primeira\nação = 2;\n");
        assert_eq!(linha_coluna(&c), (1, 1));
        posicionar(&mut c, 1, 4);
        assert_eq!(linha_coluna(&c), (2, 5));
        c.perform(Action::Move(Motion::End));
        assert_eq!(linha_coluna(&c), (2, 10));
        posicionar(&mut c, 9, 99);
        // O "\n" final não cria linha no editor: além do fim para no fim da última.
        assert_eq!(linha_coluna(&c), (2, 10), "linha e coluna além do fim param no fim");
    }

    #[test]
    fn todos_os_modulos_usam_o_texto_do_editor_no_modulo_aberto() {
        let mut app = app_com_codigo("sheet.set('A1', 1);");
        app.macro_catalogo.macros = vec![
            modulo("Macro1", "guardado sem a palavra"),
            modulo("Contas", "log(1);\nsheet.clear('B1');"),
        ];
        let _ = app.evento_codigo_macro(EventoCodigo::Abrir(false));
        let _ = app.evento_codigo_macro(EventoCodigo::Consulta("sheet".into()));
        let todos = app.ocorrencias_em_todos();
        assert_eq!(todos.len(), 2);
        assert_eq!((todos[0].0.as_str(), todos[0].1.linha), ("Macro1", 0));
        assert_eq!((todos[1].0.as_str(), todos[1].1.linha), ("Contas", 1));
        assert_eq!(todos[1].2, "sheet.clear('B1');");
    }
}
