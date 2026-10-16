//! Painel Iced: extração no motor Sheet, desenho no Charts compartilhado.
use super::*;
mod editor;
mod excel;
mod painel;
mod posicionamento;
mod seletor_tipo;
use posicionamento::{Alca, Quadro};
use feather_charts::{Grafico, Tipo, TipografiaGrafico};
use feathersheet_core::celulas::refs::{format_ref, parse_range_ref};
use feathersheet_core::grafico::chart::{AggregateFn, ChartConfig, ChartRange, ChartType};
use feather_ui::seletor_cor::{self, Evento as EventoCor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TipoUi {
    #[default]
    Colunas,
    ColunasEmpilhadas,
    Colunas100,
    Barras,
    BarrasEmpilhadas,
    Barras100,
    Linhas,
    Linha3d,
    Degraus,
    DegrausEmpilhados,
    Degraus100,
    Area,
    AreaDegraus,
    AreaDegrausEmpilhada,
    AreaDegraus100,
    Pizza,
    Dispersao,
    Bolhas,
    Combinado,
    MapaCalor,
    Cascata,
    Histograma,
    Radar,
    Candlestick,
    Gantt,
}
impl TipoUi {
    #[cfg(test)]
    const TODOS: [Self; 25] = [
        Self::Colunas,
        Self::ColunasEmpilhadas,
        Self::Colunas100,
        Self::Barras,
        Self::BarrasEmpilhadas,
        Self::Barras100,
        Self::Linhas,
        Self::Linha3d,
        Self::Degraus,
        Self::DegrausEmpilhados,
        Self::Degraus100,
        Self::Area,
        Self::AreaDegraus,
        Self::AreaDegrausEmpilhada,
        Self::AreaDegraus100,
        Self::Pizza,
        Self::Dispersao,
        Self::Bolhas,
        Self::Combinado,
        Self::MapaCalor,
        Self::Cascata,
        Self::Histograma,
        Self::Radar,
        Self::Candlestick,
        Self::Gantt,
    ];
    fn motor(self) -> ChartType {
        match self {
            Self::Colunas => ChartType::Column,
            Self::ColunasEmpilhadas => ChartType::ColumnStacked,
            Self::Colunas100 => ChartType::ColumnStacked100,
            Self::Barras => ChartType::Bar,
            Self::BarrasEmpilhadas => ChartType::BarStacked,
            Self::Barras100 => ChartType::BarStacked100,
            Self::Linhas => ChartType::Line,
            Self::Linha3d => ChartType::Line3d,
            Self::Degraus => ChartType::StepLine,
            Self::DegrausEmpilhados => ChartType::StepLineStacked,
            Self::Degraus100 => ChartType::StepLineStacked100,
            Self::Area => ChartType::Area,
            Self::AreaDegraus => ChartType::StepArea,
            Self::AreaDegrausEmpilhada => ChartType::StepAreaStacked,
            Self::AreaDegraus100 => ChartType::StepAreaStacked100,
            Self::Pizza => ChartType::Column,
            Self::Dispersao => ChartType::Scatter,
            Self::Bolhas => ChartType::Bubble,
            Self::Combinado => ChartType::Combo,
            Self::MapaCalor => ChartType::Heatmap,
            Self::Cascata => ChartType::Waterfall,
            Self::Histograma => ChartType::Histogram,
            Self::Radar => ChartType::Radar,
            Self::Candlestick => ChartType::Candlestick,
            Self::Gantt => ChartType::Gantt,
        }
    }
    /// Os sete tipos anteriores continuam usando o caminho vetorial validado.
    /// Os demais são despachados por featheroffice-charts::motor.
    fn avancado(self) -> bool {
        !matches!(self, Self::Colunas | Self::Barras | Self::Linhas | Self::Area | Self::Degraus | Self::Pizza | Self::Dispersao
            | Self::ColunasEmpilhadas | Self::Colunas100 | Self::BarrasEmpilhadas | Self::Barras100)
    }
}
impl std::fmt::Display for TipoUi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Colunas => "Colunas",
            Self::ColunasEmpilhadas => "Colunas empilhadas",
            Self::Colunas100 => "Colunas 100%",
            Self::Barras => "Barras",
            Self::BarrasEmpilhadas => "Barras empilhadas",
            Self::Barras100 => "Barras 100%",
            Self::Linhas => "Linhas",
            Self::Linha3d => "Linha 3D",
            Self::Degraus => "Degraus",
            Self::DegrausEmpilhados => "Degraus empilhados",
            Self::Degraus100 => "Degraus 100%",
            Self::Area => "Áreas",
            Self::AreaDegraus => "Área degraus",
            Self::AreaDegrausEmpilhada => "Área empilhada",
            Self::AreaDegraus100 => "Área 100%",
            Self::Pizza => "Pizza",
            Self::Dispersao => "Dispersão",
            Self::Bolhas => "Bolhas",
            Self::Combinado => "Combinado",
            Self::MapaCalor => "Mapa de calor",
            Self::Cascata => "Cascata",
            Self::Histograma => "Histograma",
            Self::Radar => "Radar",
            Self::Candlestick => "Candlestick",
            Self::Gantt => "Gantt",
        })
    }
}
#[derive(Debug, Clone)]
pub enum Evento {
    Abrir,
    Fechar,
    FecharSeletor,
    Cursor(iced::Point),
    Arrastar(String),
    Redimensionar(String, Alca),
    Soltar,
    Remover(String),
    Editar(String),
    /// Botão direito no gráfico: menu com Editar e Excluir.
    Contexto(String),
    FecharContexto,
    Inserir,
    Estilo(CampoEstilo, String),
    AbrirSeletorCor(AlvoCorGrafico),
    SeletorCor(EventoCor),
    Visibilidade(CampoVisivel, bool),
    CorSerie(usize, String),
    NomeSerie(usize, String),
    Intervalo(String),
    Titulo(String),
    Tipo(TipoUi),
    AlternarSeletor,
    Cabecalho(bool),
    Categorias(bool),
    Colunas(bool),
    Agrupar(bool),
    Serie(String),
    /// Aba Configurar: coluna das categorias (`None` = sem categorias).
    CategoriaColuna(Option<u32>),
    /// Mostrar/ocultar a série de origem `n`.
    SerieVisivel(u32, bool),
    /// Conta da série `n` (`None` = não resumir: vale para todas).
    SerieConta(u32, Option<AggregateFn>),
    /// Nome na legenda da série de origem `n`.
    SerieNome(u32, String),
    /// Intervalo = a seleção atual da planilha.
    UsarSelecao,
    /// Copia o gráfico (menu do botão direito ou Ctrl+C).
    Copiar(String),
    /// Cola o gráfico copiado na aba ativa.
    Colar,
    /// Copia e cola na hora, deslocado (Ctrl+D).
    Duplicar(String),
    /// Troca entre as abas Configurar e Personalizar do painel.
    AbaPainel(painel::AbaPainel),
    Atualizar,
    Exportar,
    Exportado(Result<bool, String>),
    /// Painel Formatar gráfico: abre/fecha um grupo do acordeão.
    Grupo(painel::Grupo),
    Selecionar(painel::Selecao),
    /// Abre (ou fecha, com `None`) a paleta de uma amostra de cor.
    CorMenu(Option<painel::AlvoCor>),
    MaisCores(painel::AlvoCor),
    /// Cor escolhida na paleta (`None` = automática).
    Cor(painel::AlvoCor, Option<String>),
    Formatar(painel::Mudanca),
}
/// Alvo do seletor HSV compartilhado; independente das cores das células.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlvoCorGrafico {
    Principal,
    Fundo,
    Serie(usize),
    /// Qualquer cor do estilo novo (painel Formatar gráfico).
    Estilo(painel::AlvoCor),
}

#[derive(Debug, Clone, Copy)]
pub enum CampoEstilo {
    Subtitulo,
    Unidade,
    Cor,
    Fonte,
    EixoX,
    EixoY,
    Fundo,
    Largura,
    Altura,
}
impl CampoEstilo {
    /// Chave do campo para juntar letras seguidas num passo de Ctrl+Z.
    fn chave(self) -> &'static str {
        match self {
            Self::Subtitulo => "subtitulo",
            Self::Unidade => "unidade",
            Self::Cor => "cor",
            Self::Fonte => "fonte",
            Self::EixoX => "eixo_x",
            Self::EixoY => "eixo_y",
            Self::Fundo => "fundo",
            Self::Largura => "largura",
            Self::Altura => "altura",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum CampoVisivel {
    Titulo,
    Legenda,
    Eixos,
    Valores,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Aparencia {
    eixo_x: String,
    eixo_y: String,
    fundo: String,
    largura: String,
    altura: String,
    cores: std::collections::BTreeMap<usize, String>,
    nomes: std::collections::BTreeMap<usize, String>,
    /// Nome na legenda escolhido na aba Configurar, pela ordem da coluna
    /// (ou linha) de origem: não muda quando outra série é ocultada.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    nomes_origem: std::collections::BTreeMap<u32, String>,
    subtitulo: String,
    unidade: String,
    cor: String,
    fonte: String,
    titulo: bool,
    legenda: bool,
    eixos: bool,
    valores: bool,
    /// Formato dos rótulos de valor (`moeda:2`, `decimal:0`, `percentual:1`);
    /// vazio = o padrão do gráfico. Vem dos gráficos do `.xlsx`.
    formato: String,
    /// Estilo completo do painel Formatar gráfico; `None` = gráfico antigo,
    /// desenhado como antes até a primeira edição no painel novo.
    #[serde(skip_serializing_if = "Option::is_none")]
    estilo: Option<feather_charts::EstiloGrafico>,
}
impl Default for Aparencia {
    fn default() -> Self {
        Self {
            eixo_x: String::new(),
            eixo_y: String::new(),
            fundo: "#ffffff".into(),
            largura: "480".into(),
            altura: "300".into(),
            cores: Default::default(),
            nomes: Default::default(),
            nomes_origem: Default::default(),
            subtitulo: String::new(),
            unidade: String::new(),
            cor: "#4285f4".into(),
            fonte: "12".into(),
            titulo: true,
            legenda: true,
            eixos: true,
            valores: false,
            formato: String::new(),
            estilo: None,
        }
    }
}
#[derive(Default)]
pub(super) struct Estado {
    pub(super) aberto: bool,
    pub(super) cursor: iced::Point,
    /// X do botão Gráfico no instante da abertura; não acompanha o cursor depois.
    pub(super) ancora_seletor_x: f32,
    arraste: Option<(String, u32, iced::Point, f64, f64)>,
    /// Menu do botão direito aberto: (gráfico, x, y na janela).
    pub(super) contexto: Option<(String, f32, f32)>,
    /// Último clique num gráfico (o Iced 0.13 não tem duplo clique no `mouse_area`).
    ultimo_clique: Option<(String, std::time::Instant)>,
    redimensionamento: Option<(String, u32, iced::Point, Quadro, Alca)>,
    selecionado: Option<String>,
    id: Option<String>,
    aparencia: Aparencia,
    /// Um único seletor de cor aberto por vez, dentro do painel lateral.
    pub(super) cor_ativa: Option<AlvoCorGrafico>,
    cor_hsva: [f32; 4],
    cor_hex: String,
    intervalo: String,
    titulo: String,
    tipo: TipoUi,
    pub(super) seletor_aberto: bool,
    cabecalho: bool,
    categorias: bool,
    colunas: bool,
    agrupar: bool,
    /// Coluna (ou linha) das categorias, relativa ao começo do intervalo.
    coluna_categoria: u32,
    /// Séries ocultas, pela ordem de origem (sem a coluna de categoria).
    ocultas: Vec<u32>,
    /// Conta de cada série quando as categorias iguais são juntadas.
    contas: std::collections::BTreeMap<u32, AggregateFn>,
    /// Aba de onde vêm os dados (`sourceSheet` no gráfico do `.xlsx`).
    aba_id: Option<u32>,
    /// Aba onde o gráfico está desenhado e gravado (pode não ser a dos dados).
    aba_grafico: Option<u32>,
    /// Último campo de texto editado: letras seguidas no mesmo campo viram
    /// um passo só de Ctrl+Z.
    ultimo_campo: Option<&'static str>,
    serie: Option<String>,
    series: Vec<String>,
    /// Gráfico copiado (Ctrl+C ou menu), com `sourceSheet` preenchido para
    /// a cópia colada em outra aba continuar lendo os mesmos dados.
    copiado: Option<serde_json::Value>,
    /// Interface do painel Formatar gráfico (grupo aberto, seleções).
    pub(super) painel: painel::EstadoPainel,
    svg: Option<String>,
    mensagem: String,
}
/// Uma única política visual para a prévia e o gráfico embutido.
/// Toda a extração/agregação permanece no Sheet; Charts recebe apenas dados,
/// cor e rótulo, sem recontar valores ou alterar os arquivos .fshz.
fn aplicar_aparencia_grafico(g: &mut Grafico, a: &Aparencia) -> Result<(), String> {
    let primaria = cor_rgb(&a.cor)
        .ok_or("Use uma cor hexadecimal, como #4285f4.")?;
    // Para pizza, o editor já reduziu para uma série; os índices de
    // aparência referem-se à série VISÍVEL, tal como na prévia legada.
    for (i, serie) in g.series.iter_mut().enumerate() {
        if i == 0 {
            serie.cor = primaria;
        }
        if let Some(cor) = a.cores.get(&i) {
            serie.cor = cor_rgb(cor)
                .ok_or("Use cores hexadecimais, como #4285f4.")?;
        }
        if let Some(nome) = a.nomes.get(&i) {
            serie.nome = nome.clone();
        }
    }
    Ok(())
}

/// Nomes da legenda escolhidos na aba Configurar (pela série de origem).
fn aplicar_nomes_origem(c: &mut feather_charts::ContratoGrafico, a: &Aparencia) {
    for (serie, meta) in c.grafico.series.iter_mut().zip(&c.metadados_series) {
        if let Some(nome) = a.nomes_origem.get(&meta.indice_origem).filter(|n| !n.trim().is_empty()) {
            serie.nome = nome.clone();
        }
    }
}

/// O mesmo dos nomes de origem para os tipos avançados (empilhados,
/// cascata…), que desenham direto do `ChartData`.
fn renomear_series_dados(d: &mut feathersheet_core::grafico::chart::ChartData, ocultas: &[u32], a: &Aparencia) {
    let origens = (0u32..).filter(|o| !ocultas.contains(o));
    for (serie, o) in d.series.iter_mut().zip(origens) {
        if let Some(nome) = a.nomes_origem.get(&o).filter(|n| !n.trim().is_empty()) {
            serie.name = nome.clone();
        }
    }
}

/// Colunas/barras empilhadas: o tipo base e como empilhar. Desenhadas pelo
/// desenho com estilo (fundo, título, eixos), não pelo motor dos avançados.
fn empilhamento(t: ChartType) -> Option<(ChartType, feather_charts::estilo::Empilhar)> {
    use feather_charts::estilo::Empilhar as E;
    Some(match t {
        ChartType::ColumnStacked => (ChartType::Column, E::Empilhado),
        ChartType::ColumnStacked100 => (ChartType::Column, E::Cem),
        ChartType::BarStacked => (ChartType::Bar, E::Empilhado),
        ChartType::BarStacked100 => (ChartType::Bar, E::Cem),
        _ => return None,
    })
}

/// A aparência com o empilhamento do tipo (cria o estilo se ainda não há).
fn aparencia_empilhada(a: &Aparencia, emp: feather_charts::estilo::Empilhar) -> Aparencia {
    let mut a = a.clone();
    a.estilo.get_or_insert_with(Default::default).empilhar = emp;
    a
}

fn preparar_svg(g: &Grafico, a: &Aparencia) -> Result<String, String> {
    preparar_svg_em(g, a, 800., 450.)
}

/// `moeda:2` → `FormatoNumero::Moeda(2)`; vazio ou inválido = o padrão.
fn formato_numero(f: &str) -> Option<feather_charts::FormatoNumero> {
    use feather_charts::FormatoNumero as F;
    let (tipo, casas) = f.split_once(':')?;
    let casas = casas.parse::<u8>().ok()?.min(6);
    Some(match tipo { "moeda" => F::Moeda(casas), "percentual" => F::Percentual(casas), "decimal" => F::Decimal(casas), _ => return None })
}

/// O SVG do gráfico em `largura` x `altura` (unidades da cena).
fn preparar_svg_em(g: &Grafico, a: &Aparencia, largura: f32, altura: f32) -> Result<String, String> {
    if let Some(e) = &a.estilo {
        return g.exportar_svg_formatado(largura, altura, e)
            .map_err(|_| "Não foi possível desenhar o gráfico com essas opções.".to_string());
    }
    let tamanho = a
        .fonte
        .parse::<f32>()
        .ok()
        .filter(|v| (4.0..=32.0).contains(v))
        .ok_or("O tamanho da fonte deve estar entre 4 e 32.")?;
    let fundo = cor_rgb(&a.fundo).ok_or("Use uma cor de fundo hexadecimal, como #ffffff.")?;
    let mut saida = g
        .exportar_svg_anotado(
            largura,
            altura,
            &TipografiaGrafico {
                formato: formato_numero(&a.formato).unwrap_or(feather_charts::FormatoNumero::Decimal(1)),
                subtitulo: a.subtitulo.clone(),
                unidade: a.unidade.clone(),
                tamanho,
                mostrar_titulo: a.titulo,
                mostrar_legenda: a.legenda,
                mostrar_eixos: a.eixos,
                mostrar_valores: a.valores,
                ..Default::default()
            },
        )
        .map_err(|_| "Não foi possível desenhar o gráfico com essas opções.".to_string())?;
    let escapar = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    let inicio = saida.find('>').ok_or("SVG inválido")? + 1;
    let fundo_svg = if fundo.a == 255 {
        format!("<rect width=\"{largura}\" height=\"{altura}\" fill=\"{}\"/>", a.fundo)
    } else {
        format!(
            "<rect width=\"{largura}\" height=\"{altura}\" fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{:.3}\"/>",
            fundo.r, fundo.g, fundo.b, fundo.a as f32 / 255.0,
        )
    };
    saida.insert_str(inicio, &fundo_svg);
    if a.eixos && g.tipo != Tipo::Pizza {
        let fim = saida.rfind("</svg>").ok_or("SVG inválido")?;
        let (cx, cy, base) = (largura / 2.0, altura / 2.0, altura - 3.0);
        saida.insert_str(fim,&format!("<text x=\"{cx}\" y=\"{base}\" text-anchor=\"middle\" font-size=\"12\">{}</text><text x=\"12\" y=\"{cy}\" transform=\"rotate(-90 12 {cy})\" text-anchor=\"middle\" font-size=\"12\">{}</text>",escapar(&a.eixo_x),escapar(&a.eixo_y)));
    }
    // Famílias explícitas evitam rótulos invisíveis quando o SVG não resolve a fonte genérica.
    saida = saida.replace(
        "font-family=\"sans-serif\"",
        "font-family=\"Noto Sans, DejaVu Sans, Liberation Sans, Arial, sans-serif\"",
    );
    Ok(saida)
}
/// Aceita a mesma codificação #RRGGBB/#RRGGBBAA do seletor compartilhado.
/// Alfa segue até a Cena vetorial, sem converter RGB em outra paleta.
fn cor_rgb(s: &str) -> Option<feather_draw::Cor> {
    if !seletor_cor::hex_valido(s) || !s.trim().starts_with('#') {
        return None;
    }
    let [r, g, b, a] = seletor_cor::rgba_de(s);
    Some(feather_draw::Cor::rgba(r, g, b, a))
}
impl App {
    /// Aplica um clique/arrasto do colorpicker sem afetar o colorpicker das
    /// células. A prévia é recalculada pelo mesmo motor Charts de sempre.
    /// Sincroniza HSV quando o usuário digita hexadecimal com a paleta aberta.
    fn sincronizar_campo_hex_grafico(&mut self, alvo: AlvoCorGrafico, hex: &str) {
        if self.graficos.cor_ativa != Some(alvo) { return; }
        self.graficos.cor_hex = hex.to_owned();
        if seletor_cor::hex_valido(hex) {
            self.graficos.cor_hsva = seletor_cor::rgba_para_hsva(
                seletor_cor::rgba_de(hex), self.graficos.cor_hsva[0],
            );
        }
    }

    fn aplicar_cor_seletor_grafico(&mut self, nova: String) {
        let Some(alvo) = self.graficos.cor_ativa else { return; };
        // O picker compartilhado possui a opção Sem cor; em gráficos,
        // Fundo/Principal usam seu padrão e Série volta à cor herdada.
        if let AlvoCorGrafico::Estilo(a) = alvo {
            let cor = feather_charts::estilo::CorEstilo::de_texto(&nova);
            if cor.is_none() && !nova.trim().is_empty() { return; }
            let atual = cor.map_or_else(String::new, |c| c.texto());
            self.mudar_estilo(|e| { painel::definir_cor(e, a, cor); true }, Some("seletor_cor"));
            if !atual.is_empty() {
                self.graficos.cor_hex = atual.clone();
                self.graficos.cor_hsva = seletor_cor::rgba_para_hsva(seletor_cor::rgba_de(&atual), self.graficos.cor_hsva[0]);
            }
            return;
        }
        if nova.trim().is_empty() {
            match alvo {
                AlvoCorGrafico::Principal => self.graficos.aparencia.cor = "#4285f4".into(),
                AlvoCorGrafico::Fundo => self.graficos.aparencia.fundo = "#ffffff".into(),
                AlvoCorGrafico::Serie(i) => { self.graficos.aparencia.cores.remove(&i); }
                AlvoCorGrafico::Estilo(_) => {}
            }
        } else {
            if !seletor_cor::hex_valido(&nova) { return; }
            let hex = format!("#{}", nova.trim().trim_start_matches('#').to_ascii_lowercase());
            match alvo {
                AlvoCorGrafico::Principal => self.graficos.aparencia.cor = hex,
                AlvoCorGrafico::Fundo => self.graficos.aparencia.fundo = hex,
                AlvoCorGrafico::Serie(i) => { self.graficos.aparencia.cores.insert(i, hex); }
                AlvoCorGrafico::Estilo(_) => {}
            }
        }
        let atual = match alvo {
            AlvoCorGrafico::Principal => self.graficos.aparencia.cor.clone(),
            AlvoCorGrafico::Fundo => self.graficos.aparencia.fundo.clone(),
            AlvoCorGrafico::Serie(i) => self.graficos.aparencia.cores.get(&i)
                .cloned().unwrap_or_else(|| self.graficos.aparencia.cor.clone()),
            AlvoCorGrafico::Estilo(_) => return,
        };
        self.graficos.cor_hex = atual.clone();
        self.graficos.cor_hsva = seletor_cor::rgba_para_hsva(
            seletor_cor::rgba_de(&atual), self.graficos.cor_hsva[0],
        );
        self.editar_ao_vivo(Some("seletor_cor"));
    }

    fn evento_seletor_cor_grafico(&mut self, evento: EventoCor) {
        if self.graficos.cor_ativa.is_none() { return; }
        match evento {
            EventoCor::Fechar => self.graficos.cor_ativa = None,
            EventoCor::Escolhida(hex) => self.aplicar_cor_seletor_grafico(hex),
            EventoCor::SaturacaoBrilho(s, v) => {
                self.graficos.cor_hsva[1] = s.clamp(0., 1.);
                self.graficos.cor_hsva[2] = v.clamp(0., 1.);
                self.aplicar_cor_seletor_grafico(self.hex_do_seletor_grafico());
            }
            EventoCor::Matiz(h) => {
                self.graficos.cor_hsva[0] = h.rem_euclid(360.);
                self.aplicar_cor_seletor_grafico(self.hex_do_seletor_grafico());
            }
            EventoCor::Alfa(a) => {
                self.graficos.cor_hsva[3] = a.clamp(0., 1.);
                self.aplicar_cor_seletor_grafico(self.hex_do_seletor_grafico());
            }
            EventoCor::TextoHex(hex) => {
                self.graficos.cor_hex = hex.clone();
                if seletor_cor::hex_valido(&hex) {
                    self.aplicar_cor_seletor_grafico(hex);
                }
            }
            EventoCor::Canal(indice, valor) => {
                if indice < 4 {
                    if let Ok(v) = valor.parse::<u8>() {
                        let mut rgba = seletor_cor::rgba_de(&self.hex_do_seletor_grafico());
                        rgba[indice] = v;
                        self.aplicar_cor_seletor_grafico(seletor_cor::hex_de(rgba));
                    }
                }
            }
        }
    }

    fn hex_do_seletor_grafico(&self) -> String {
        let [h, s, v, a] = self.graficos.cor_hsva;
        let [r, g, b] = seletor_cor::hsv_para_rgb(h, s, v);
        seletor_cor::hex_de([r, g, b, (a * 255.).round() as u8])
    }

    /// Uma edição do painel: grava no gráfico (Ctrl+Z) e redesenha.
    fn editar_ao_vivo(&mut self, campo: Option<&'static str>) {
        self.salvar_grafico_ao_vivo(campo);
        self.atualizar_grafico();
    }

    fn atualizar_grafico(&mut self) {
        self.graficos.svg = None;
        let resultado = (|| {
            let r = parse_range_ref(self.graficos.intervalo.trim())
                .ok_or("Intervalo inválido. Use A1:C10.")?
                .normalized();
            if r.sheet.is_some() {
                return Err("Informe apenas o intervalo da aba de origem.".into());
            }
            let celulas = u64::from(r.r1 - r.r0 + 1) * u64::from(r.c1 - r.c0 + 1);
            if celulas > 200_000 {
                return Err("Selecione até 200.000 células para este painel de gráficos.".into());
            }
            let aba = (0..self.workbook.len())
                .filter_map(|i| self.workbook.aba(i))
                .find(|a| Some(a.id) == self.graficos.aba_id)
                .ok_or("A aba de origem foi removida. Abra um novo gráfico.")?;
            let pilha = empilhamento(self.graficos.tipo.motor());
            let config = ChartConfig {
                tipo: pilha.map_or(self.graficos.tipo.motor(), |p| p.0),
                title: self.graficos.titulo.clone(),
                range: ChartRange {
                    r0: r.r0,
                    c0: r.c0,
                    r1: r.r1,
                    c1: r.c1,
                },
                series_in_columns: self.graficos.colunas,
                has_headers: self.graficos.cabecalho,
                has_category_labels: self.graficos.categorias,
                category_column_index: Some(self.graficos.coluna_categoria),
                aggregate: self.graficos.agrupar,
                aggregate_fn: Some(AggregateFn::Sum),
                aggregate_by_series: self.graficos.contas.clone(),
                style: feathersheet_core::grafico::chart::ChartStyle {
                    hidden_series: self.graficos.ocultas.clone(),
                    ..Default::default()
                },
                ..Default::default()
            };
            // Novo motor Rust compartilhado: tipos especializados do chart.ts.
            // Mantém os sete tipos já consolidados no Charts legado, bem como
            // todo o processamento de células, agrupamento e FSHZ no Sheet.
            if self.graficos.tipo.avancado() {
                let mut dados = feathersheet_core::grafico::chart::extract_chart_data(
                    &aba.sheet, &config
                );
                renomear_series_dados(&mut dados, &config.style.hidden_series, &self.graficos.aparencia);
                self.graficos.series = dados.series.iter().enumerate()
                    .map(|(i,s)| format!("{} — {}",i+1,s.name)).collect();
                self.graficos.painel.categorias = dados.categories.clone();
                let svg = feathersheet_io::grafico_migrado::renderizar_dados(
                    &config,&dados,800.,450.
                )?;
                self.graficos.mensagem = dados.aviso.unwrap_or_else(|| format!(
                    "{} categorias, {} séries. Tipo {} em Rust.",
                    dados.categories.len(),dados.series.len(),self.graficos.tipo
                ));
                return Ok(svg);
            }
            let mut contrato =
                feathersheet_io::grafico_compartilhado::extrair_grafico(&aba.sheet, &config)
                    .map_err(|_| "Selecione uma tabela com categorias e valores numéricos. Se a tabela não tem cabeçalhos, desative essa opção.".to_string())?;
            aplicar_nomes_origem(&mut contrato, &self.graficos.aparencia);
            self.graficos.series = contrato
                .grafico
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{} — {}", i + 1, s.nome))
                .collect();
            if self.graficos.tipo == TipoUi::Pizza {
                let i = self
                    .graficos
                    .serie
                    .as_ref()
                    .and_then(|s| self.graficos.series.iter().position(|v| v == s))
                    .unwrap_or(0);
                self.graficos.serie = self.graficos.series.get(i).cloned();
                contrato.grafico.series = vec![contrato.grafico.series[i].clone()];
                contrato.grafico.tipo = Tipo::Pizza;
            }
            aplicar_aparencia_grafico(
                &mut contrato.grafico, &self.graficos.aparencia
            )?;
            self.graficos.painel.cores_base = contrato.grafico.series.iter().map(|s| s.cor).collect();
            self.graficos.painel.categorias = contrato.grafico.categorias.clone();
            let mut avisos = contrato.avisos;
            if let Some(a) = contrato.amostra {
                avisos.push(format!(
                    "Visualização amostrada: {} de {} pontos.",
                    a.usados, a.disponiveis
                ));
            }
            self.graficos.mensagem = if avisos.is_empty() {
                format!(
                    "{} categorias, {} séries.",
                    contrato.grafico.categorias.len(),
                    contrato.grafico.series.len()
                )
            } else {
                avisos.join("; ")
            };
            match pilha {
                Some((_, emp)) => preparar_svg(&contrato.grafico, &aparencia_empilhada(&self.graficos.aparencia, emp)),
                None => preparar_svg(&contrato.grafico, &self.graficos.aparencia),
            }
        })();
        match resultado {
            Ok(svg) => self.graficos.svg = Some(svg),
            Err(e) => self.graficos.mensagem = e,
        }
        // Gráfico já gravado: a prévia é o desenho dele na planilha (o mesmo
        // caminho da camada, com o fundo e as cores do Excel quando houver).
        if let Some(svg) = self.svg_do_grafico_em_edicao() {
            if self.graficos.svg.is_none() {
                self.graficos.mensagem.clear();
            }
            self.graficos.svg = Some(svg);
        }
    }

    fn svg_do_grafico_em_edicao(&self) -> Option<String> {
        let id = self.graficos.id.as_deref()?;
        let aba = self.workbook.aba(self.indice_aba_grafico()?)?;
        let v = feathersheet_io::graficos_salvos::listar(&aba.sheet).ok()?
            .into_iter().find(|v| v["id"].as_str() == Some(id))?;
        let dados = v["sourceSheet"].as_str()
            .and_then(|n| self.workbook.indice_por_nome(n))
            .and_then(|i| self.workbook.aba(i))
            .map_or(&aba.sheet, |a| &a.sheet);
        editor::svg_do_grafico_salvo(dados, &v)
    }
    /// Menu do botão direito do gráfico aberto: (id, x, y na janela).
    pub(in crate::app) fn graficos_contexto(&self) -> Option<(String, f32, f32)> {
        self.graficos.contexto.clone()
    }

    /// Tira a seleção do gráfico (clique numa célula ou numa forma).
    pub(in crate::app) fn desselecionar_grafico(&mut self) {
        self.graficos.selecionado = None;
        self.graficos.contexto = None;
    }

    /// Delete com um gráfico selecionado (e o editor fechado): exclui.
    pub(in crate::app) fn apagar_grafico_selecionado(&mut self) -> bool {
        match self.graficos.selecionado.clone().filter(|_| !self.graficos.aberto) {
            Some(id) => {
                let _ = self.evento_grafico(Evento::Remover(id));
                true
            }
            None => false,
        }
    }

    /// Guarda o gráfico `id` da aba ativa para colar depois.
    fn copiar_grafico(&mut self, id: &str) -> bool {
        let Some(aba) = self.workbook.aba(self.folha) else { return false };
        let Some(mut v) = feathersheet_io::graficos_salvos::listar(&aba.sheet).ok()
            .and_then(|l| l.into_iter().find(|v| v["id"].as_str() == Some(id))) else { return false };
        if v["sourceSheet"].as_str().is_none() {
            v["sourceSheet"] = serde_json::json!(aba.sheet.name);
        }
        self.graficos.copiado = Some(v);
        self.status = "Gráfico copiado. Ctrl+V cola.".into();
        true
    }

    /// Cola o gráfico copiado na aba ativa, 24 px abaixo e à direita do
    /// original (ou da última cópia), com id novo; um Ctrl+Z.
    fn colar_grafico(&mut self) -> bool {
        let Some(mut v) = self.graficos.copiado.clone() else { return false };
        let id = format!("native-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
        v["id"] = serde_json::json!(id);
        for k in ["x", "y"] {
            let n = v["position"][k].as_f64().unwrap_or(80.0) + 24.0;
            v["position"][k] = serde_json::json!(n);
        }
        // A próxima colagem sai mais deslocada, como no Excel.
        self.graficos.copiado = Some(v.clone());
        // Dados na própria aba: sem sourceSheet, como os criados aqui.
        if let Some(aba) = self.workbook.aba(self.folha) {
            if v["sourceSheet"].as_str() == Some(aba.sheet.name.as_str()) {
                if let Some(o) = v.as_object_mut() { o.remove("sourceSheet"); }
            }
        }
        let Some(aba) = self.workbook.aba_mut(self.folha) else { return false };
        match feathersheet_io::graficos_salvos::mesclado(&aba.sheet, &id, v) {
            Ok(novo) => {
                aba.sheet.alterar_graficos(Some(novo), false);
                self.alterado = true;
                self.graficos.selecionado = Some(id);
                self.invalidar_cache_objetos();
                self.status = "Gráfico colado (Ctrl+Z desfaz).".into();
                true
            }
            Err(e) => {
                self.status = e;
                false
            }
        }
    }

    /// Outra coisa foi copiada (forma): Ctrl+V não cola mais o gráfico.
    pub(in crate::app) fn graficos_esquecer_copia(&mut self) {
        self.graficos.copiado = None;
    }

    /// Há gráfico copiado para o "Colar" do menu.
    pub(in crate::app) fn grafico_copiado(&self) -> bool {
        self.graficos.copiado.is_some()
    }

    /// Ctrl+C/V/D com gráfico selecionado (painel fechado). Devolve se usou.
    pub(in crate::app) fn atalho_grafico(&mut self, letra: &str) -> bool {
        let sel = self.graficos.selecionado.clone().filter(|_| !self.graficos.aberto);
        match (letra, sel) {
            ("c", Some(id)) => self.copiar_grafico(&id),
            ("d", Some(id)) => self.copiar_grafico(&id) && self.colar_grafico(),
            ("v", _) if self.graficos.copiado.is_some() => self.colar_grafico(),
            // Copiar células (ou forma) depois de um gráfico: Ctrl+V volta a colar aquilo.
            ("c" | "x", None) => {
                self.graficos.copiado = None;
                false
            }
            _ => false,
        }
    }

    pub(super) fn evento_grafico(&mut self, event: Evento) -> Task<Msg> {
        match event {
            Evento::Abrir => {
                if self.graficos.seletor_aberto {
                    self.graficos.seletor_aberto = false;
                    return Task::none();
                }
                self.confirmar();
                self.menu_aberto = false;
                let mut area = self.area_selecionada();
                if area.r0 == area.r1 && area.c0 == area.c1 {
                    if let Some(aba) = self.workbook.aba_mut(self.folha) {
                        area = aba.sheet.regiao_da_tabela(area.r0, area.c0);
                    }
                }
                // Captura a posição do clique ANTES de recriar o estado.
                // A assinatura global do mouse usa coordenadas da janela.
                let cursor_abertura = self.graficos.cursor;
                let copiado = self.graficos.copiado.take();
                self.graficos = Estado {
                    copiado,
                    aberto: false,
                    cursor: cursor_abertura,
                    ancora_seletor_x: cursor_abertura.x,
                    seletor_aberto: true,
                    intervalo: format_ref(&area),
                    titulo: "Gráfico".into(),
                    cabecalho: self.workbook.aba(self.folha).is_some_and(|a| {
                        // Se a primeira linha já contém números, não é um
                        // cabeçalho. Ex.: "a 1" / "b 2" são duas categorias.
                        if area.c1 <= area.c0 { return false; }
                        let inicial = a.sheet.get_raw(area.r0, area.c0 + 1);
                        let texto = inicial.trim();
                        !texto.is_empty()
                            && texto.parse::<f64>().is_err()
                            && texto.replace(',', ".").parse::<f64>().is_err()
                    }),
                    categorias: true,
                    colunas: true,
                    aba_id: self.workbook.aba(self.folha).map(|a| a.id),
                    ..Default::default()
                };
                self.atualizar_grafico();
                return Task::none();
            }
            Evento::Cursor(p) => {
                self.graficos.cursor = p;
                let zoom = (self.config.zoom as f64 / 100.).max(0.01);
                if let Some((id, aba_id, origem, x, y)) = self.graficos.arraste.clone() {
                    if let Some(aba) = self.workbook.aba_mut(self.folha).filter(|a| a.id == aba_id)
                    {
                        if let Ok(lista) = feathersheet_io::graficos_salvos::listar(&aba.sheet) {
                            if let Some(v) = lista.iter().find(|v| v["id"].as_str() == Some(&id)) {
                                let mut pos = v["position"].clone();
                                pos["x"] =
                                    serde_json::json!((x + f64::from(p.x - origem.x) / zoom).max(0.));
                                pos["y"] =
                                    serde_json::json!((y + f64::from(p.y - origem.y) / zoom).max(0.));
                                if feathersheet_io::graficos_salvos::salvar(
                                    &mut aba.sheet,
                                    &id,
                                    serde_json::json!({"position":pos}),
                                )
                                .is_ok()
                                {
                                    self.alterado = true;
                                }
                            }
                        }
                    }
                }
                if let Some((id, aba_id, origem, quadro, alca)) =
                    self.graficos.redimensionamento.clone()
                {
                    if let Some(aba) = self.workbook.aba_mut(self.folha).filter(|a| a.id == aba_id) {
                        let novo = quadro.redimensionar(
                            alca,
                            f64::from(p.x - origem.x) / zoom,
                            f64::from(p.y - origem.y) / zoom,
                        );
                        // Preservar x/y, campos e metadados existentes.
                        // Ler ANTES de pedir &mut aba.sheet para o salvamento.
                        let mut pos = feathersheet_io::graficos_salvos::listar(&aba.sheet)
                            .ok()
                            .and_then(|lista| lista.into_iter()
                                .find(|v| v["id"].as_str() == Some(id.as_str()))
                                .map(|v| v["position"].clone()))
                            .unwrap_or_else(|| serde_json::json!({"x":80.,"y":80.}));
                        pos["x"] = serde_json::json!(novo.x);
                        pos["y"] = serde_json::json!(novo.y);
                        pos["width"] = serde_json::json!(novo.largura);
                        pos["height"] = serde_json::json!(novo.altura);
                        if feathersheet_io::graficos_salvos::salvar(
                            &mut aba.sheet, &id, serde_json::json!({"position": pos}),
                        ).is_ok() {
                            self.alterado = true;
                            // O editor lateral mantém as dimensões da alça,
                            // evitando restaurar o tamanho antigo ao salvar.
                            if self.graficos.id.as_deref() == Some(id.as_str()) {
                                self.graficos.aparencia.largura = format!("{:.0}", novo.largura);
                                self.graficos.aparencia.altura = format!("{:.0}", novo.altura);
                            }
                        }
                    }
                }
            }
            Evento::Arrastar(id) => {
                self.graficos.contexto = None;
                // Segundo clique no mesmo gráfico logo depois: abre o editor.
                let duplo = self.graficos.ultimo_clique.as_ref()
                    .is_some_and(|(u, t)| *u == id && t.elapsed().as_millis() < 450);
                self.graficos.ultimo_clique = (!duplo).then(|| (id.clone(), std::time::Instant::now()));
                if duplo {
                    return self.evento_grafico(Evento::Editar(id));
                }
                self.graficos.selecionado = Some(id.clone());
                self.graficos.redimensionamento = None;
                if let Some(aba) = self.workbook.aba(self.folha) {
                    if let Ok(lista) = feathersheet_io::graficos_salvos::listar(&aba.sheet) {
                        if let Some(v) = lista.iter().find(|v| v["id"].as_str() == Some(&id)) {
                            self.graficos.arraste = Some((
                                id,
                                aba.id,
                                self.graficos.cursor,
                                v["position"]["x"].as_f64().unwrap_or(80.),
                                v["position"]["y"].as_f64().unwrap_or(80.),
                            ));
                        }
                    }
                }
            }
            Evento::Redimensionar(id, alca) => {
                self.graficos.selecionado = Some(id.clone());
                self.graficos.arraste = None;
                if let Some(aba) = self.workbook.aba(self.folha) {
                    if let Ok(lista) = feathersheet_io::graficos_salvos::listar(&aba.sheet) {
                        if let Some(v) = lista.iter().find(|v| v["id"].as_str() == Some(&id)) {
                            if let Some(quadro) = Quadro::da_posicao(&v["position"]) {
                                self.graficos.redimensionamento = Some((
                                    id, aba.id, self.graficos.cursor, quadro, alca,
                                ));
                            }
                        }
                    }
                }
            }
            Evento::Contexto(id) => {
                self.graficos.selecionado = Some(id.clone());
                self.graficos.arraste = None;
                self.graficos.redimensionamento = None;
                self.graficos.contexto = Some((id, self.ponteiro_x, self.ponteiro_y));
            }
            Evento::FecharContexto => self.graficos.contexto = None,
            Evento::Remover(id) => {
                self.graficos.contexto = None;
                if let Some(aba) = self.workbook.aba_mut(self.folha) {
                    let removido = feathersheet_io::graficos_salvos::sem(&aba.sheet, &id)
                        .map(|novo| novo.is_some_and(|n| aba.sheet.alterar_graficos(Some(n), false)));
                    match removido {
                        Ok(true) => {
                            self.alterado = true;
                            self.graficos.aberto = false;
                            self.graficos.id = None;
                            self.graficos.selecionado = None;
                            self.status = "Gráfico excluído.".into();
                        }
                        Ok(false) => {}
                        Err(e) => self.graficos.mensagem = e,
                    }
                }
            }
            Evento::Soltar => {
                self.graficos.arraste = None;
                self.graficos.redimensionamento = None;
            }
            Evento::Fechar => {
                self.graficos.aberto = false;
                self.graficos.seletor_aberto = false;
                self.graficos.cor_ativa = None;
            }
            Evento::FecharSeletor => {
                self.graficos.seletor_aberto = false;
            }
            Evento::Editar(id) => {
                self.graficos.contexto = None;
                self.graficos.arraste = None;
                self.graficos.selecionado = Some(id.clone());
                return self.editar_grafico_salvo(&id);
            }
            Evento::Inserir => {
                return self.inserir_grafico();
            }
            Evento::AbrirSeletorCor(alvo) => {
                if self.graficos.cor_ativa == Some(alvo) {
                    self.graficos.cor_ativa = None;
                } else {
                    let cor = match alvo {
                        AlvoCorGrafico::Principal => self.graficos.aparencia.cor.clone(),
                        AlvoCorGrafico::Fundo => self.graficos.aparencia.fundo.clone(),
                        AlvoCorGrafico::Serie(i) => self.graficos.aparencia.cores.get(&i)
                            .cloned().unwrap_or_else(|| self.graficos.aparencia.cor.clone()),
                        AlvoCorGrafico::Estilo(a) => {
                            let e = self.graficos.aparencia.estilo.clone().unwrap_or_default();
                            painel::cor_de(&e, a).map_or_else(|| "#4285f4".to_owned(), |c| c.texto())
                        }
                    };
                    self.graficos.cor_hsva = seletor_cor::rgba_para_hsva(
                        seletor_cor::rgba_de(&cor), self.graficos.cor_hsva[0],
                    );
                    self.graficos.cor_hex = cor;
                    self.graficos.cor_ativa = Some(alvo);
                }
            }
            Evento::SeletorCor(evento) => self.evento_seletor_cor_grafico(evento),
            e @ (Evento::Grupo(_) | Evento::Selecionar(_) | Evento::CorMenu(_) | Evento::MaisCores(_)
                | Evento::Cor(..) | Evento::Formatar(_)) => return self.evento_painel(e),
            Evento::CorSerie(i, valor) => {
                self.sincronizar_campo_hex_grafico(AlvoCorGrafico::Serie(i), &valor);
                self.graficos.aparencia.cores.insert(i, valor);
                self.editar_ao_vivo(Some("cor_serie"));
            }
            Evento::NomeSerie(i, valor) => {
                self.graficos.aparencia.nomes.insert(i, valor);
                self.editar_ao_vivo(Some("nome_serie"));
            }
            Evento::Estilo(campo, valor) => {
                match campo {
                    CampoEstilo::Cor =>
                        self.sincronizar_campo_hex_grafico(AlvoCorGrafico::Principal, &valor),
                    CampoEstilo::Fundo =>
                        self.sincronizar_campo_hex_grafico(AlvoCorGrafico::Fundo, &valor),
                    _ => {}
                }
                match campo {
                    CampoEstilo::Subtitulo => self.graficos.aparencia.subtitulo = valor,
                    CampoEstilo::Unidade => self.graficos.aparencia.unidade = valor,
                    CampoEstilo::Cor => self.graficos.aparencia.cor = valor,
                    CampoEstilo::Fonte => self.graficos.aparencia.fonte = valor,
                    CampoEstilo::EixoX => self.graficos.aparencia.eixo_x = valor,
                    CampoEstilo::EixoY => self.graficos.aparencia.eixo_y = valor,
                    CampoEstilo::Fundo => self.graficos.aparencia.fundo = valor,
                    CampoEstilo::Largura => self.graficos.aparencia.largura = valor,
                    CampoEstilo::Altura => self.graficos.aparencia.altura = valor,
                }
                self.editar_ao_vivo(Some(campo.chave()));
            }
            Evento::Visibilidade(campo, valor) => {
                match campo {
                    CampoVisivel::Titulo => self.graficos.aparencia.titulo = valor,
                    CampoVisivel::Legenda => self.graficos.aparencia.legenda = valor,
                    CampoVisivel::Eixos => self.graficos.aparencia.eixos = valor,
                    CampoVisivel::Valores => self.graficos.aparencia.valores = valor,
                }
                self.editar_ao_vivo(None);
            }
            Evento::Intervalo(s) => {
                self.graficos.intervalo = s;
                self.editar_ao_vivo(Some("intervalo"));
            }
            Evento::Titulo(s) => {
                self.graficos.titulo = s;
                self.editar_ao_vivo(Some("titulo"));
            }
            Evento::Tipo(t) => {
                self.graficos.cor_ativa = None;
                self.graficos.tipo = t;
                self.graficos.aberto = true;
                self.graficos.seletor_aberto = false;
                self.editar_ao_vivo(None);
                if self.graficos.id.is_none() && self.graficos.svg.is_some() {
                    return self.inserir_grafico();
                }
            }
            Evento::AlternarSeletor => {
                // Galeria de tipos e picker de cor não competem por foco.
                self.graficos.cor_ativa = None;
                self.graficos.seletor_aberto = !self.graficos.seletor_aberto;
            }
            Evento::Cabecalho(v) => {
                self.graficos.cabecalho = v;
                self.editar_ao_vivo(None);
            }
            Evento::Categorias(v) => {
                self.graficos.categorias = v;
                self.editar_ao_vivo(None);
            }
            Evento::Colunas(v) => {
                self.graficos.colunas = v;
                self.editar_ao_vivo(None);
            }
            Evento::Agrupar(v) => {
                self.graficos.agrupar = v;
                self.editar_ao_vivo(None);
            }
            Evento::Serie(s) => {
                self.graficos.serie = Some(s);
                self.editar_ao_vivo(None);
            }
            Evento::CategoriaColuna(c) => {
                let g = &mut self.graficos;
                g.categorias = c.is_some();
                if let Some(c) = c {
                    // As séries contam sem a coluna de categoria: trocar a
                    // categoria muda a ordem, então mostrar e contas recomeçam.
                    if g.coluna_categoria != c {
                        g.ocultas.clear();
                        g.contas.clear();
                        g.aparencia.nomes_origem.clear();
                    }
                    g.coluna_categoria = c;
                }
                self.editar_ao_vivo(None);
            }
            Evento::SerieVisivel(n, v) => {
                let total = self.series_da_origem().len() as u32;
                let g = &mut self.graficos;
                if g.tipo == TipoUi::Pizza {
                    // Pizza desenha uma série: escolher uma oculta as outras.
                    g.ocultas = (0..total).filter(|&i| i != n).collect();
                    g.serie = None;
                } else if v {
                    g.ocultas.retain(|&i| i != n);
                } else if total.saturating_sub(g.ocultas.len() as u32) > 1 {
                    g.ocultas.push(n);
                    g.ocultas.sort_unstable();
                } else {
                    g.mensagem = "O gráfico precisa de pelo menos uma série.".into();
                    return Task::none();
                }
                self.editar_ao_vivo(None);
            }
            Evento::SerieConta(n, conta) => {
                let g = &mut self.graficos;
                match conta {
                    Some(c) => {
                        g.agrupar = true;
                        g.contas.insert(n, c);
                    }
                    None => {
                        g.agrupar = false;
                        g.contas.clear();
                    }
                }
                self.editar_ao_vivo(None);
            }
            Evento::SerieNome(n, nome) => {
                if nome.is_empty() {
                    self.graficos.aparencia.nomes_origem.remove(&n);
                } else {
                    self.graficos.aparencia.nomes_origem.insert(n, nome);
                }
                self.editar_ao_vivo(Some("serie_nome"));
            }
            Evento::Copiar(id) => {
                self.graficos.contexto = None;
                self.copiar_grafico(&id);
            }
            Evento::Colar => {
                self.graficos.contexto = None;
                self.colar_grafico();
            }
            Evento::Duplicar(id) => {
                self.graficos.contexto = None;
                if self.copiar_grafico(&id) {
                    self.colar_grafico();
                }
            }
            Evento::AbaPainel(aba) => {
                self.graficos.painel.aba = aba;
                self.graficos.painel.cor_menu = None;
                self.graficos.cor_ativa = None;
            }
            Evento::UsarSelecao => {
                let area = self.area_selecionada();
                self.graficos.intervalo = format_ref(&area);
                self.graficos.aba_id = self.workbook.aba(self.folha).map(|a| a.id);
                self.graficos.ocultas.clear();
                self.graficos.contas.clear();
                self.graficos.aparencia.nomes_origem.clear();
                self.editar_ao_vivo(None);
            }
            Evento::Atualizar => {
                self.confirmar();
                self.atualizar_grafico();
            }
            Evento::Exportar => {
                if let Some(conteudo) = self.graficos.svg.clone() {
                    return Task::perform(
                        async move {
                            let Some(p) = rfd::AsyncFileDialog::new()
                                .add_filter("SVG", &["svg"])
                                .set_file_name("grafico.svg")
                                .save_file()
                                .await
                            else {
                                return Ok(false);
                            };
                            std::fs::write(p.path(), conteudo).map_err(|e| e.to_string())?;
                            Ok(true)
                        },
                        |r| Msg::Grafico(Evento::Exportado(r)),
                    );
                }
            }
            Evento::Exportado(r) => {
                self.graficos.mensagem = match r {
                    Ok(true) => "Gráfico exportado em SVG".into(),
                    Ok(false) => "Exportação cancelada".into(),
                    Err(e) => format!("Erro ao exportar: {e}"),
                }
            }
        }
        Task::none()
    }
}
#[cfg(test)]
mod testes;
