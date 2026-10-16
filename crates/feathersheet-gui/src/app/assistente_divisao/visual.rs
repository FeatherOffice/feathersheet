//! Layout do assistente Texto em Colunas (Iced): fiel ao mockup aprovado.
//! Sem sidebar. Dados em tabela, quatro círculos no rodapé e cor do tema.
//! Prévia ilustrativa fica visivelmente identificada e nunca é convertida.
use super::*;
use iced::{Background, Border};
use iced::widget::{checkbox, pick_list, scrollable};

const MAX_AMOSTRA: u32 = 24;
const COLUNAS_VISIVEIS: usize = 4;
const TIPOS: [&str; 8] = [
    "Geral", "Número", "Texto", "Data DMA", "Data MDA",
    "Data AMD", "Porcentagem", "Ignorar",
];

fn titulo(s: &str) -> Element<'static, Msg> {
    text(s.to_owned()).size(17)
        .font(iced::Font {weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT})
        .color(COR_TEXTO()).into()
}
fn destaque(s: &str) -> Element<'static, Msg> {
    text(s.to_owned()).size(13)
        .font(iced::Font {weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT})
        .color(COR_TEXTO()).into()
}
fn legenda(s: &str) -> Element<'static, Msg> {
    text(s.to_owned()).size(12).color(COR_TEXTO_MUTED()).into()
}
fn linha_fina() -> Element<'static, Msg> {
    container(Space::with_height(Length::Fixed(1.)))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(COR_BORDA_SUAVE())),
            ..Default::default()
        }).into()
}
fn estilo_cartao(ativo: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, estado| {
        let fundo = if ativo {COR_AZUL_FUNDO()} else if matches!(
            estado, button::Status::Hovered | button::Status::Pressed
        ) {COR_HOVER()} else {COR_PAINEL()};
        button::Style {
            background: Some(Background::Color(fundo)),
            text_color: COR_TEXTO(),
            border: Border {
                color: if ativo {COR_AZUL_ACENTO()} else {COR_BORDA_SUAVE()},
                width: if ativo {1.4} else {1.0},
                radius: 9.0.into(),
            },
            ..Default::default()
        }
    }
}
/// Botão primário acompanha o acento do sistema também em hover/pressed.
fn estilo_primario(_theme:&iced::Theme,status:button::Status)->button::Style {
    let cor=COR_AZUL_ACENTO();
    let k=match status {
        button::Status::Hovered=>0.92,
        button::Status::Pressed=>0.84,
        button::Status::Disabled=>0.60,
        _=>1.0,
    };
    let bg=iced::Color {r:cor.r*k,g:cor.g*k,b:cor.b*k,a:cor.a};
    button::Style{
        background:Some(Background::Color(bg)),
        text_color:feather_ui::acessibilidade::texto_sobre(bg),
        border:Border{radius:6.0.into(),..Default::default()},
        ..Default::default()
    }
}
fn estilo_etapa(ativo: bool, concluida: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_,estado| {
        let realce=ativo||concluida;
        let fundo=if realce {COR_AZUL_ACENTO()} else if matches!(
            estado,button::Status::Hovered|button::Status::Pressed
        ) {COR_HOVER()} else {COR_PAINEL_FAIXA()};
        button::Style {
            background: Some(Background::Color(fundo)),
            text_color: if realce {feather_ui::acessibilidade::texto_sobre(fundo)}
                else {COR_TEXTO()},
            border: Border {
                color: if realce {COR_AZUL_ACENTO()} else {COR_BORDA_SUAVE()},
                width: 1., radius: 25.0.into(),
            },
            ..Default::default()
        }
    }
}
/// Rádio de verdade (widget do Iced) com a cor do app; sem rótulo, pois o
/// texto fica ao lado no próprio cartão ou na linha do destino.
fn radio_app<V: Copy + Eq + 'static>(
    valor: V, selecionado: V, msg: impl Fn(V) -> Msg + 'static,
) -> Element<'static, Msg> {
    iced::widget::radio("", valor, Some(selecionado), msg)
        .size(17.).spacing(0.)
        .style(estilo_radio)
        .into()
}
fn estilo_radio(_t:&iced::Theme, status:iced::widget::radio::Status)->iced::widget::radio::Style{
    let selecionado=match status {
        iced::widget::radio::Status::Active{is_selected}
        | iced::widget::radio::Status::Hovered{is_selected} => is_selected,
    };
    iced::widget::radio::Style{
        background:Background::Color(COR_PAINEL_FAIXA()),
        dot_color:COR_AZUL_ACENTO(),
        border_width:1.,
        border_color:if selecionado {COR_AZUL_ACENTO()} else {COR_TEXTO_MUTED()},
        text_color:Some(COR_TEXTO()),
    }
}
/// Opção compacta de separador: rádio, nome e o caractere, numa linha.
fn seletor(
    nome: &'static str, simbolo: &'static str, chave: &'static str, atual: &'static str,
) -> Element<'static, Msg> {
    let mut linha = iced::widget::row![
        radio_app(chave, atual, |v| Msg::AssistenteDivisao(Evento::Separador(v))),
        text(nome).size(12),
    ].spacing(7).align_y(Alignment::Center);
    if !simbolo.is_empty() {
        linha = linha.push(text(simbolo).size(11).color(COR_TEXTO_MUTED()));
    }
    button(linha)
        .padding([6,10])
        .style(estilo_cartao(chave == atual))
        .on_press(Msg::AssistenteDivisao(Evento::Separador(chave))).into()
}
fn label_tipo(t: &TipoColunaDivisao) -> &'static str {
    match t {
        TipoColunaDivisao::Geral => "Geral",
        TipoColunaDivisao::Texto => "Texto",
        TipoColunaDivisao::Numero => "Número",
        TipoColunaDivisao::Data(FormatoDeData::DiaMesAno) => "Data DMA",
        TipoColunaDivisao::Data(FormatoDeData::MesDiaAno) => "Data MDA",
        TipoColunaDivisao::Data(FormatoDeData::AnoMesDia) => "Data AMD",
        TipoColunaDivisao::Porcentagem => "Porcentagem",
        TipoColunaDivisao::Pular => "Ignorar",
    }
}
/// O exemplo é usado somente quando a seleção não contém dado algum.
fn exemplo() -> Vec<(u32, String)> {
    [
        "Nome;Idade;Cidade",
        "João;25;Vitória",
        "Maria;30;Serra",
        "Pedro;28;Vila Velha",
        "Ana;22;Cariacica",
    ].into_iter().enumerate().map(|(i,s)| (i as u32,s.to_owned())).collect()
}
fn tem_cabecalho(linhas: &[Vec<String>]) -> bool {
    if linhas.len() < 2 || linhas[0].len() < 2 { return false; }
    let primeira = &linhas[0];
    primeira.iter().all(|campo|
        !campo.trim().is_empty() && campo.trim().parse::<f64>().is_err()
    ) && linhas.iter().skip(1).take(4).any(|linha|
        linha.iter().enumerate().any(|(i,v)| i < primeira.len()
            && v.trim().parse::<f64>().is_ok())
    )
}
fn celula_conteudo<'a>(valor: String, selecionada: bool) -> Element<'a, Msg> {
    container(text(valor).size(12))
        .padding([7,10]).width(Length::FillPortion(1))
        .style(move |_| container::Style {
            background: Some(Background::Color(
                if selecionada {COR_AZUL_FUNDO()} else {COR_PAINEL()}
            )),
            border: Border {color:COR_BORDA_SUAVE(),width:0.45,..Default::default()},
            ..Default::default()
        }).into()
}
fn celula_indice<'a>(n: String) -> Element<'a, Msg> {
    container(text(n).size(11).color(COR_TEXTO_MUTED()))
        .padding([7,8]).width(Length::Fixed(38.))
        .style(|_| container::Style {
            background:Some(Background::Color(COR_PAINEL_FAIXA())),
            border:Border {color:COR_BORDA_SUAVE(),width:0.45,..Default::default()},
            ..Default::default()
        }).into()
}

impl App {
    pub(super) fn linhas_divisao(&self, e: &Estado) -> (Vec<(u32,String)>, bool) {
        let mut linhas=Vec::new();
        if let Some(aba)=self.workbook.aba(e.origem) {
            let fim=e.area.r1.min(aba.sheet.used_rect().r1);
            if e.area.r0<=fim {
                let limite=fim.min(e.area.r0.saturating_add(MAX_AMOSTRA-1));
                for r in e.area.r0..=limite {
                    let valor=texto_valor(self.workbook.valor(e.origem,r,e.area.c0));
                    linhas.push((r,valor));
                }
            }
        }
        if linhas.iter().any(|(_,v)|!v.is_empty()) {
            (linhas,false)
        } else {
            (exemplo(),true)
        }
    }

    /// Grade reutilizada nas etapas 1–4; conteúdo vem da planilha real
    /// ou de um exemplo explicitamente rotulado, nunca misturados.
    fn grade_previa(&self,e:&Estado,dividido:bool)->Element<'_,Msg> {
        let (linhas,ilustrativo)=self.linhas_divisao(e);
        let recortes:Vec<Vec<String>>=linhas.iter().map(|(_,original)|
            if dividido {e.partes(original)} else {vec![original.clone()]}
        ).collect();
        let max_cols=recortes.iter().map(Vec::len).max().unwrap_or(1).max(e.tipos.len());
        let colunas=if dividido {max_cols.min(COLUNAS_VISIVEIS).max(1)} else {1};
        let com_cabecalho=dividido && tem_cabecalho(&recortes);
        let edicao=e.etapa==3 && dividido;
        let primeira=if com_cabecalho {1}else{0};

        let mut tabela=iced::widget::column![].spacing(0);
        let mut cabecalho=iced::widget::row![
            celula_indice(String::new())
        ].spacing(0);
        for c in 0..colunas {
            let nome=if dividido {
                if com_cabecalho && !edicao {
                    recortes[0].get(c).cloned().unwrap_or_else(||format!("Coluna {}",c+1))
                }else{format!("Coluna {}",c+1)}
            }else{"Dados originais".to_owned()};
            let selecionada=dividido && e.selecionada==c;
            let conteudo:Element<'_,Msg>=if edicao {
                iced::widget::column![
                    text(nome).size(12),
                    pick_list(
                        TIPOS,Some(label_tipo(&e.tipo(c))),
                        move |tipo|Msg::AssistenteDivisao(Evento::TipoNaColuna(c,tipo))
                    ).text_size(11).padding([5,7]).width(Length::Fill),
                ].spacing(4).into()
            }else{
                text(nome).size(12).into()
            };
            // PickList não pode ficar dentro de um Button clicável: o botão
            // capturaria o mouse e impediria abrir o menu de formatos.
            let cabeca: Element<'_,Msg>=if edicao {
                container(conteudo).padding([7,8])
                    .width(Length::FillPortion(1))
                    .style(|_|container::Style {
                        background:Some(Background::Color(COR_PAINEL_FAIXA())),
                        border:Border {color:COR_BORDA_SUAVE(),width:0.5,
                            ..Default::default()},
                        ..Default::default()
                    }).into()
            }else{
                button(conteudo)
                    .width(Length::FillPortion(1))
                    .padding([10,10])
                    .style(estilo_cartao(selecionada))
                    .on_press(Msg::AssistenteDivisao(Evento::Coluna(c)))
                    .into()
            };
            cabecalho=cabecalho.push(cabeca);
        }
        tabela=tabela.push(cabecalho);
        for (i, partes) in recortes.iter().enumerate().skip(primeira) {
            let mut linha=iced::widget::row![
                celula_indice(format!("{}",if ilustrativo {i+1-primeira} else {
                    linhas[i].0 as usize+1
                }))
            ].spacing(0);
            for c in 0..colunas {
                let valor=partes.get(c).cloned().unwrap_or_default();
                let visivel=valor.chars().take(90).collect::<String>();
                linha=linha.push(celula_conteudo(visivel,dividido && e.selecionada==c && edicao));
            }
            tabela=tabela.push(linha);
        }
        let titulo_previa=if dividido {"Visualização dos dados"}else{"Visualização dos dados selecionados"};
        let note=if ilustrativo {
            "Exemplo ilustrativo — selecione células preenchidas para visualizar seus dados."
        }else{"A prévia usa somente a seleção; a conversão aplica-se à seleção inteira."};
        iced::widget::column![
            iced::widget::row![
                destaque(titulo_previa),
                Space::with_width(Length::Fill),
                legenda(&format!("{} linha(s) de amostra",
                    recortes.len().saturating_sub(primeira))),
            ].align_y(Alignment::Center),
            container(scrollable(tabela)
                .height(Length::Fixed(if edicao {205.} else {174.}))
                .width(Length::Fill))
                .width(Length::Fill)
                .style(|_|container::Style{
                    background:Some(Background::Color(COR_PAINEL())),
                    border:Border{color:COR_BORDA_SUAVE(),width:1.,radius:7.0.into()},
                    ..Default::default()
                }),
            if ilustrativo {
                legenda(note)
            }else if max_cols>colunas {
                legenda(&format!("Exibindo {} de {} colunas · {}",colunas,max_cols,note))
            }else{
                legenda(note)
            },
        ].spacing(8).width(Length::Fill).into()
    }

    fn tela_modo(&self,e:&Estado)->Element<'_,Msg> {
        let cartao=|modo:Modo,nome:&str,descricao:&str,icon:&str| {
            let ativo=e.modo==modo;
            button(iced::widget::row![
                radio_app(modo,e.modo,|v|Msg::AssistenteDivisao(Evento::Modo(v))),
                container(feather_ui::icones::icone(icon,23.))
                    .padding(8)
                    .style(|_|container::Style{
                        background:Some(Background::Color(COR_PAINEL_FAIXA())),
                        border:Border{radius:6.0.into(),..Default::default()},
                        ..Default::default()
                    }),
                iced::widget::column![
                    destaque(nome),
                    legenda(descricao),
                ].spacing(6).width(Length::Fill),
            ].spacing(12).align_y(Alignment::Center))
                .padding([14,14]).width(Length::FillPortion(1))
                .style(estilo_cartao(ativo))
                .on_press(Msg::AssistenteDivisao(Evento::Modo(modo)))
        };
        iced::widget::column![
            titulo("Escolha como seus dados estão delimitados"),
            legenda("Selecione o método de separação. A pré-visualização será atualizada."),
            iced::widget::row![
                cartao(Modo::Delimitado,"Delimitado",
                    "Vírgulas e tabulações separam os campos.","columns"),
                cartao(Modo::LarguraFixa,"Largura fixa",
                    "Campos organizados em larguras definidas.","arrows-horizontal"),
            ].spacing(12),
            self.grade_previa(e,false),
        ].spacing(14).width(Length::Fill).into()
    }

    fn tela_separadores<'a>(&'a self,e:&'a Estado)->Element<'a,Msg> {
        let controles:Element<'a,Msg>=if e.modo==Modo::LarguraFixa {
            iced::widget::column![
                legenda("Largura fixa de cada coluna"),
                text_input("Quantidade de caracteres", &e.largura)
                    .on_input(|s|Msg::AssistenteDivisao(Evento::Largura(s)))
                    .width(Length::Fixed(250.)).padding([9,12]).size(12),
            ].spacing(8).into()
        }else {
            let atual=e.separador_atual();
            iced::widget::column![
                iced::widget::row![
                    seletor("Ponto e vírgula",";",";",atual),
                    seletor("Vírgula",",",",",atual),
                    seletor("Tabulação","\\t","Tab",atual),
                    seletor("Espaço","( )","Espaço",atual),
                    seletor("Outro","","Outro",atual),
                    text_input("Caractere", &e.outro)
                        .on_input(|s|Msg::AssistenteDivisao(Evento::Outro(s)))
                        .width(Length::Fixed(90.)).padding([6,8]).size(11),
                ].spacing(6).align_y(Alignment::Center).wrap(),
                iced::widget::row![
                    checkbox("Considerar delimitadores consecutivos como um só",e.consecutivos)
                        .size(14).text_size(12)
                        .on_toggle(|s|Msg::AssistenteDivisao(Evento::Consecutivos(s))),
                    Space::with_width(Length::Fill),
                    legenda("Qualificador de texto:"),
                    pick_list(
                        ["Aspas duplas","Aspas simples","Nenhum"],Some(e.qualificador),
                        |v|Msg::AssistenteDivisao(Evento::Qualificador(v))
                    ).width(Length::Fixed(147.)).text_size(11).padding([7,8]),
                ].spacing(9).align_y(Alignment::Center),
            ].spacing(11).into()
        };
        iced::widget::column![
            titulo(if e.modo==Modo::Delimitado {
                "Escolha os delimitadores"
            }else{"Defina a largura fixa"}),
            legenda(if e.modo==Modo::Delimitado {"Escolha o caractere que separa os campos."} else {"Informe quantos caracteres tem cada campo."}),
            controles,
            self.grade_previa(e,true),
        ].spacing(12).width(Length::Fill).into()
    }

    fn tela_formato(&self,e:&Estado)->Element<'_,Msg> {
        iced::widget::column![
            titulo("Defina o formato de cada coluna"),
            legenda("Cada coluna tem seu próprio formato. Selecione Ignorar para não importá-la."),
            self.grade_previa(e,true),
            iced::widget::row![
                button(if e.avancado {"Ocultar opções avançadas"}else{"Avançado..."})
                    .padding([8,13]).style(estilo_botao_secundario)
                    .on_press(Msg::AssistenteDivisao(Evento::Avancado)),
                Space::with_width(Length::Fill),
                container(iced::widget::row![
                    feather_ui::icones::icone("info-circle",18.)
                        .style(|_,_| iced::widget::svg::Style{color:Some(COR_AZUL_ACENTO())}),
                    legenda(if e.avancado {
                        "Datas: DMA (dia/mês/ano), MDA (mês/dia/ano) ou AMD (ano/mês/dia)."
                    } else {
                        "Geral converte números e mantém os demais valores como texto."
                    }),
                ].spacing(9).align_y(Alignment::Center))
                    .padding([9,11]).width(Length::Fixed(385.))
                    .style(|_|container::Style {
                        background:Some(Background::Color(COR_AZUL_FUNDO())),
                        border:Border{color:COR_BORDA_SUAVE(),width:0.8,radius:7.0.into()},
                        ..Default::default()
                    }),
            ].align_y(Alignment::Center),
        ].spacing(14).width(Length::Fill).into()
    }
    fn destino_input<'a>(
        &self, e:&'a Estado, nova:bool
    )->Element<'a, Msg> {
        // Como no mockup: campo da opção não selecionada permanece inativo.
        if e.nova_aba != nova {
            return container(legenda(if nova {&e.nome_aba}else{&e.destino}))
                .padding([10,12]).width(Length::Fill)
                .style(|_|container::Style {
                    background:Some(Background::Color(COR_PAINEL_FAIXA())),
                    border:Border{color:COR_BORDA_SUAVE(),width:1.,radius:6.0.into()},
                    ..Default::default()
                }).into()
        }
        if nova {
            text_input("Dados_Separados",&e.nome_aba)
                .on_input(|s|Msg::AssistenteDivisao(Evento::NomeAba(s)))
                .padding([8,10]).size(12).width(Length::Fill).into()
        }else{
            iced::widget::row![
                text_input("=$A$1",&e.destino)
                    .on_input(|s|Msg::AssistenteDivisao(Evento::Destino(s)))
                    .padding([8,10]).size(12).width(Length::Fill),
                button(feather_ui::icones::icone("table",17.))
                    .style(estilo_botao_secundario).padding([9,11])
                    .on_press(Msg::AssistenteDivisao(Evento::UsarCelulaAtual)),
            ].spacing(6).align_y(Alignment::Center).into()
        }
    }
    fn tela_destino<'a>(&'a self,e:&'a Estado)->Element<'a,Msg> {
        let (linhas,_)=self.linhas_divisao(e);
        let total=linhas.iter().map(|(_,s)|e.partes(s).len()).max().unwrap_or(0)
            .max(e.tipos.len());
        let mantidas=(0..total).filter(|&i|e.tipo(i)!=TipoColunaDivisao::Pular).count();
        iced::widget::column![
            titulo("Escolha o destino dos dados"),
            legenda("Escolha onde as colunas convertidas serão colocadas."),
            iced::widget::column![
                iced::widget::row![
                    radio_app(false,e.nova_aba,|v|Msg::AssistenteDivisao(Evento::NovaAba(v))),
                    container(text("Planilha existente:").size(12)).width(Length::Fixed(150.)),
                    self.destino_input(e,false),
                ].spacing(9).align_y(Alignment::Center),
                iced::widget::row![
                    radio_app(true,e.nova_aba,|v|Msg::AssistenteDivisao(Evento::NovaAba(v))),
                    container(text("Nova planilha").size(12)).width(Length::Fixed(150.)),
                    self.destino_input(e,true),
                ].spacing(9).align_y(Alignment::Center),
            ].spacing(8).width(Length::Fill),
            iced::widget::column![
                destaque("Resumo"),
                legenda(&format!("Os dados serão divididos em {total} colunas; {mantidas} serão importadas.")),
            ].spacing(4),
            self.grade_previa(e,true),
        ].spacing(12).width(Length::Fill).into()
    }

    fn rodape_divisao(&self,e:&Estado)->Element<'_,Msg> {
        let mut etapas=iced::widget::row![].spacing(7).align_y(Alignment::Center);
        for n in 1..=4u8 {
            if n>1 {
                etapas=etapas.push(text("—").size(10).color(COR_BORDA()));
            }
            let conteudo: Element<'static,Msg> = if n<e.etapa {
                feather_ui::icones::icone("circle-check",14.)
                    .style(|_,_| iced::widget::svg::Style{
                        color:Some(feather_ui::acessibilidade::texto_sobre(COR_AZUL_ACENTO())),
                    }).into()
            }else{
                text(n.to_string()).size(12).into()
            };
            etapas=etapas.push(button(conteudo)
                .padding([7,11])
                .style(estilo_etapa(n==e.etapa,n<e.etapa))
                .on_press(Msg::AssistenteDivisao(Evento::IrPara(n))));
        }
        let mut acoes=iced::widget::row![
            button("Cancelar").padding([8,17]).style(estilo_botao_secundario)
                .on_press(Msg::AssistenteDivisao(Evento::Cancelar)),
            Space::with_width(Length::Fill),
            etapas,
            Space::with_width(Length::Fill),
        ].spacing(10).align_y(Alignment::Center);
        if e.etapa==1 {
            acoes=acoes.push(button("Pular").padding([8,17])
                .style(estilo_botao_secundario)
                .on_press(Msg::AssistenteDivisao(Evento::Pular)));
        }else{
            acoes=acoes.push(button(iced::widget::row![
                feather_ui::icones::icone("chevron-left",14.),
                text("Voltar"),
            ].spacing(6).align_y(Alignment::Center)).padding([8,15])
                .style(estilo_botao_secundario)
                .on_press(Msg::AssistenteDivisao(Evento::Voltar)));
        }
        let rotulo_avancar: Element<'static,Msg> = if e.etapa==4 {
            text("Concluir").into()
        }else{
            iced::widget::row![
                text("Avançar"),
                feather_ui::icones::icone("chevron-right",14.)
                    .style(|_,_| iced::widget::svg::Style{
                        color:Some(feather_ui::acessibilidade::texto_sobre(COR_AZUL_ACENTO())),
                    }),
            ].spacing(6).align_y(Alignment::Center).into()
        };
        acoes=acoes.push(button(rotulo_avancar)
            .padding([8,18]).style(estilo_primario)
            .on_press(Msg::AssistenteDivisao(if e.etapa==4 {
                Evento::Concluir
            }else{Evento::Avancar})));
        acoes.into()
    }
    pub(in crate::app) fn view_assistente_divisao(&self) -> Element<'_,Msg> {
        let Some(e)=self.assistente_divisao.as_ref() else {
            return text("Assistente fechado").into()
        };
        if let Some(progresso)=e.progresso_conversao() {
            // Durante o milhão de linhas não reconstruímos a prévia nem
            // consultamos fórmulas em cada frame. Apenas progresso e Cancelar.
            // O valor suavizado: a barra desliza em vez de pular por lote.
            let porcentagem=e.percentual_exibido();
            return container(iced::widget::column![
                titulo("Convertendo texto em colunas"),
                legenda("A planilha está sendo processada em lotes sem bloquear o aplicativo."),
                iced::widget::progress_bar(0.0..=100.0, porcentagem)
                    .width(Length::Fill),
                destaque(&progresso),
                legenda("Cancelar reverte todos os lotes já gravados."),
                button("Cancelar conversão")
                    .padding([10, 16])
                    .style(estilo_botao_secundario)
                    .on_press(Msg::AssistenteDivisao(Evento::Cancelar)),
            ].spacing(22).width(Length::Fill).align_x(Alignment::Center))
                .padding([48, 35])
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(estilo_container_topo).into();
        }
        let corpo=match e.etapa {
            1=>self.tela_modo(e),2=>self.tela_separadores(e),
            3=>self.tela_formato(e),_=>self.tela_destino(e),
        };
        let aviso:Element<'_,Msg>=if e.erro.is_empty(){
            Space::with_height(Length::Fixed(0.)).into()
        }else{
            let mut linha=iced::widget::row![
                text(e.erro.clone()).size(12).color(COR_TEXTO()).width(Length::Fill),
            ].spacing(10).align_y(Alignment::Center);
            if e.conflito.is_some() {
                linha=linha.push(button("Substituir").padding([6,14])
                    .style(estilo_primario)
                    .on_press(Msg::AssistenteDivisao(Evento::Substituir)));
            }
            container(linha)
                .padding([7,14]).width(Length::Fill)
                .style(|_|container::Style {
                    background:Some(Background::Color(COR_AZUL_FUNDO())),
                    ..Default::default()
                }).into()
        };
        container(iced::widget::column![
            container(corpo).padding([17,20]).width(Length::Fill).height(Length::Fill),
            aviso,
            linha_fina(),
            container(self.rodape_divisao(e))
                .padding([11,16]).width(Length::Fill)
                .style(|_|container::Style {
                    background:Some(Background::Color(COR_PAINEL_FAIXA())),
                    ..Default::default()
                }),
        ].spacing(0).height(Length::Fill))
            .width(Length::Fill).height(Length::Fill)
            .style(estilo_container_topo).into()
    }
}
