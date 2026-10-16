//! Interface da janela Formatar Células: abas, prévias e controles.
use super::*;

impl App {
    fn formato_linha<'a>(&self, label: &str, componente: Element<'a, Msg>) -> Element<'a, Msg> {
        iced::widget::row![
            text(label.to_string()).size(12).width(Length::Fixed(155.)),
            componente
        ].spacing(12).align_y(Alignment::Center).into()
    }

    /// Liga/desliga um efeito: o ícone diz qual é o efeito, o fundo diz se está ligado.
    fn formato_toggle(&self, e: &Estado, rotulo: &str, chave: &'static str, icone: &'static str)
        -> Element<'_, Msg>
    {
        // Com o duplo ligado, "Sublinhado" (simples) aparece desligado.
        let ativo=if chave=="underline" {
            e.booleano("underline") && !e.booleano("doubleUnderline")
        } else { e.booleano(chave) };
        button(iced::widget::row![
            feather_ui::icones::icone(icone,15.),
            text(rotulo.to_owned()).size(12),
        ].spacing(7).align_y(Alignment::Center))
            .style(move |tema, estado| {
                let mut estilo = estilo_botao_pequeno(ativo)(tema, estado);
                estilo.border.radius = 7.0.into();
                estilo.border.width = 1.0;
                estilo.border.color = if ativo { COR_AZUL_ACENTO() } else { COR_BORDA() };
                estilo
            })
            .on_press(Msg::FormatoCelula(Evento::Efeito(chave)))
            .padding([7,11]).into()
    }

    /// Rótulo pequeno acima do controle, como nos campos da Ajuda e das Configurações.
    fn formato_campo<'a>(&self, rotulo: &str, controle: Element<'a, Msg>) -> Element<'a, Msg> {
        iced::widget::column![
            text(rotulo.to_owned()).size(11.5).color(COR_TEXTO_MUTED()),
            controle,
        ].spacing(5).into()
    }

    fn formato_secao<'a>(titulo: &str) -> Element<'a, Msg> {
        text(titulo.to_owned()).size(13).font(iced::Font {
            weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT
        }).into()
    }

    fn formato_editar(&self, rotulo: &str, campo: TextoCampo, valor: &str)
        -> Element<'_, Msg>
    {
        self.formato_linha(rotulo, text_input(rotulo, valor)
            .on_input(move |valor| Msg::FormatoCelula(Evento::Texto(campo, valor)))
            .padding([7, 9]).size(12).width(Length::Fill).into())
    }

    fn formato_numero(&self, e: &Estado) -> Element<'_, Msg> {
        let escolhido = e.texto("format");
        let mut lista=iced::widget::column![].spacing(3);
        for &(rotulo, chave) in FORMATOS {
            let ligado = escolhido==chave || (escolhido.is_empty() && chave=="auto");
            lista=lista.push(button(text(rotulo).size(12))
                .width(Length::Fill)
                .padding([7,8])
                .style(estilo_botao_pequeno(ligado))
                .on_press(Msg::FormatoCelula(Evento::Formato(chave))));
        }
        let mut estilo=e.original.clone();
        for (k,v) in &e.patch {
            if matches!(v,Campo::Nulo) { estilo.remove(k); } else { estilo.insert(k.clone(),v.clone()); }
        }
        let resolvido=feathersheet_core::celulas::estilo_parcial::para_style(&estilo);
        let exemplo=feathersheet_core::celulas::style::format_value(
            &feathersheet_core::Value::Number(1234.56), Some(&resolvido));
        // Cor da seção do código ([Vermelho], [Cor10]) também no exemplo.
        let cor_exemplo=resolvido.code.as_deref()
            .and_then(|c| feathersheet_core::celulas::formato_personalizado::formatar_completo(
                &feathersheet_core::Value::Number(1234.56), c))
            .and_then(|f| f.cor)
            .map(|[r,g,b]| iced::Color::from_rgb8(r,g,b));
        // Códigos mais usados, como a lista "Personalizado" do Excel.
        let mut prontos=iced::widget::row![].spacing(4);
        for codigo in ["0", "0.00", "#,##0.00", "#,##0.00;[Vermelho]-#,##0.00", "0%", "0.00E+00",
            "# ?/?", "# ??/16", "\"R$\"* #,##0.00", "[>=1000]#,##0,\" mil\";0",
            "dd/mm/yyyy", "hh:mm", "0.00\" kWp\"", ";;;"] {
            prontos=prontos.push(button(text(codigo).size(11)).padding([4,6])
                .style(estilo_botao_pequeno(e.personalizado==codigo))
                .on_press(Msg::FormatoCelula(Evento::Texto(TextoCampo::Customizado, codigo.into()))));
        }
        iced::widget::row![
            container(lista)
                .width(Length::Fixed(206.)).padding(7).style(estilo_painel_faixa),
            iced::widget::column![
                text("Exemplo").size(12).color(COR_TEXTO_MUTED()),
                text(exemplo).size(24).color_maybe(cor_exemplo),
                self.formato_editar("Casas decimais",TextoCampo::Decimais,&e.decimais),
                text("Código personalizado").size(13),
                self.formato_editar("Formato",TextoCampo::Customizado,&e.personalizado),
                iced::widget::scrollable(prontos).direction(iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4))),
                text("Como no Excel: 0, # e ? são dígitos, \",\" separa milhar (no fim divide por mil), \".\" as casas, \
                    \"texto\" entre aspas, @ o texto da célula; seções por \";\" (positivo;negativo;zero;texto). \
                    [Vermelho] ou [Red] e [Cor1] a [Cor56] dão cor; [>=1000] é condição; * repete o caractere \
                    seguinte até encher a célula; _ reserva um espaço; # ?/? é fração; 0.00E+00 é científica. \
                    \";;;\" deixa a célula em branco.").size(11)
                    .color(COR_TEXTO_MUTED()),
            ].spacing(16).width(Length::Fill),
        ].spacing(18).into()
    }

    /// Painel compacto semelhante ao diálogo clássico: seleções à esquerda
    /// e dial interativo de orientação à direita. O estado continua em rascunho.
    fn formato_alinhamento(&self, e: &Estado) -> Element<'_, Msg> {
        const HORIZONTAL: &[&str] = &[
            "Geral", "Esquerda (recuo)", "Centro", "Direita (recuo)",
            "Justificar", "Distribuído",
        ];
        const VERTICAL: &[&str] = &[
            "Superior", "Centro", "Inferior", "Justificar", "Distribuído",
        ];
        const DIRECOES: &[&str] = &[
            "Contexto", "Da esquerda para a direita", "Da direita para a esquerda",
        ];
        let horizontal = match e.texto("align") {
            "left" => "Esquerda (recuo)",
            "center" => "Centro",
            "right" => "Direita (recuo)",
            "justify" => "Justificar",
            "distributed" => "Distribuído",
            _ => "Geral",
        };
        let vertical = match e.texto("vAlign") {
            "top" => "Superior",
            "bottom" => "Inferior",
            "justify" => "Justificar",
            "distributed" => "Distribuído",
            _ => "Centro",
        };
        let direcao = match e.texto("readingOrder") {
            "ltr" => "Da esquerda para a direita",
            "rtl" => "Da direita para a esquerda",
            _ => "Contexto",
        };
        let alinhamento_horizontal = iced::widget::pick_list(
            HORIZONTAL, Some(horizontal), |opcao| {
                let chave = match opcao {
                    "Esquerda (recuo)" => "left",
                    "Centro" => "center",
                    "Direita (recuo)" => "right",
                    "Justificar" => "justify",
                    "Distribuído" => "distributed",
                    _ => "auto",
                };
                Msg::FormatoCelula(Evento::Campo("align", if chave == "auto" {
                    Campo::Nulo
                } else { Campo::Texto(chave.into()) }))
            },
        ).padding([7, 10]).text_size(12)
            .style(estilo_menu_da_faixa).width(Length::Fill);
        let alinhamento_vertical = iced::widget::pick_list(
            VERTICAL, Some(vertical), |opcao| {
                let chave = match opcao {
                    "Superior" => "top",
                    "Inferior" => "bottom",
                    "Justificar" => "justify",
                    "Distribuído" => "distributed",
                    _ => "middle",
                };
                Msg::FormatoCelula(Evento::Campo("vAlign", Campo::Texto(chave.into())))
            },
        ).padding([7, 10]).text_size(12)
            .style(estilo_menu_da_faixa).width(Length::Fill);
        let direcao_dropdown = iced::widget::pick_list(
            DIRECOES, Some(direcao), |opcao| {
                let chave = match opcao {
                    "Da esquerda para a direita" => "ltr",
                    "Da direita para a esquerda" => "rtl",
                    _ => "",
                };
                Msg::FormatoCelula(Evento::Campo("readingOrder", if chave.is_empty() {
                    Campo::Nulo
                } else { Campo::Texto(chave.into()) }))
            },
        ).padding([7, 10]).text_size(12)
            .style(estilo_menu_da_faixa).width(Length::Fill);
        let legenda = |nome: &str| text(nome.to_owned()).size(12).color(COR_TEXTO());
        let titulo = |nome: &str| text(nome.to_owned()).size(12)
            .font(iced::Font {
                weight: iced::font::Weight::Semibold,
                ..iced::Font::DEFAULT
            }).color(COR_TEXTO());
        let divisoria = || container(Space::with_height(Length::Fixed(1.)))
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(COR_BORDA_SUAVE())),
                ..Default::default()
            });
        let recuo = text_input("0", &e.recuo)
            .on_input(|v| Msg::FormatoCelula(Evento::Texto(TextoCampo::Recuo, v)))
            .padding([7, 8]).size(12).width(Length::Fixed(58.));
        let selecao_horizontal = iced::widget::row![
            iced::widget::column![
                legenda("Horizontal:"),
                alinhamento_horizontal,
            ].spacing(5).width(Length::Fill),
            iced::widget::column![
                legenda("Recuo:"),
                recuo,
            ].spacing(5),
        ].spacing(12).align_y(Alignment::End);
        let controle_texto = iced::widget::column![
            titulo("Controle de texto"),
            self.formato_toggle(e, "Quebrar texto automaticamente", "wrap", "text-wrap"),
            self.formato_toggle(e, "Reduzir para caber", "shrinkToFit", "zoom-out"),
            button(iced::widget::row![
                feather_ui::icones::icone(if e.mesclar {"checks"} else {"minus"}, 14.),
                text("Mesclar células").size(12),
            ].spacing(7).align_y(Alignment::Center))
                .style(estilo_botao_pequeno(e.mesclar)).padding([8, 10])
                .on_press(Msg::FormatoCelula(Evento::Mesclar(!e.mesclar))),
        ].spacing(5).align_x(Alignment::Start);
        let esquerda = iced::widget::column![
            titulo("Alinhamento de texto"),
            selecao_horizontal,
            iced::widget::column![
                legenda("Vertical:"),
                alinhamento_vertical,
            ].spacing(5),
            divisoria(),
            controle_texto,
            divisoria(),
            titulo("Da direita para a esquerda"),
            iced::widget::column![
                legenda("Direção do texto:"),
                direcao_dropdown,
            ].spacing(5),
        ].spacing(11).width(Length::Fill);

        let graus=e.angulo.parse::<i16>().unwrap_or(0).clamp(-90, 90);
        let direita = iced::widget::column![
            titulo("Orientação"),
            super::orientacao_visual::controle(graus, e.booleano("textVertical")),
            iced::widget::row![
                text("Graus:").size(12).color(COR_TEXTO_MUTED()),
                text_input("0", &e.angulo)
                    .on_input(|v| Msg::FormatoCelula(Evento::Texto(TextoCampo::Angulo, v)))
                    .padding([6, 8]).size(12).width(Length::Fixed(62.)),
                text("°").size(12),
            ].spacing(8).align_y(Alignment::Center),
            text("Clique no arco ou informe de −90° a 90°.")
                .size(11).color(COR_TEXTO_MUTED()),
        ].spacing(11).width(Length::Fixed(237.));

        iced::widget::row![
            container(esquerda).padding(14).width(Length::Fill)
                .style(estilo_painel_faixa),
            container(direita).padding(14).width(Length::Fixed(265.))
                .style(estilo_painel_faixa),
        ].spacing(14).align_y(Alignment::Start).into()
    }

    fn formato_fonte(&self, e: &Estado) -> Element<'_, Msg> {
        // Estilo como controle segmentado: uma opção ligada por vez.
        let mut estilos=iced::widget::row![].spacing(2);
        for (rotulo,i) in [("Regular",0),("Negrito",1),("Itálico",2),("Negrito e itálico",3)] {
            let selecionado=match i {
                0=>!e.booleano("bold")&&!e.booleano("italic"),
                1=>e.booleano("bold")&&!e.booleano("italic"),
                2=>!e.booleano("bold")&&e.booleano("italic"),
                _=>e.booleano("bold")&&e.booleano("italic"),
            };
            estilos=estilos.push(button(text(rotulo).size(12).width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center))
                .width(Length::FillPortion(if i == 3 { 5 } else { 3 }))
                .padding([7,8])
                .style(move |tema, estado| {
                    let mut estilo = estilo_botao_pequeno(selecionado)(tema, estado);
                    estilo.border.radius = 6.0.into();
                    estilo
                })
                .on_press(Msg::FormatoCelula(Evento::FonteEstilo(i))));
        }
        let estilos = container(estilos).padding(3).width(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(COR_PAINEL())),
                border: iced::Border { color: COR_BORDA(), width: 1.0, radius: 8.0.into() },
                ..Default::default()
            });
        let cor=if seletor_cor::hex_valido(&e.cor_texto) {
            let [r,g,b,a]=seletor_cor::rgba_de(&e.cor_texto);
            iced::Color::from_rgba8(r,g,b,a as f32/255.)
        } else { COR_TEXTO() };
        let mut amostra=text("AaBbCc 123").size(
            e.tamanho.parse::<f32>().unwrap_or(16.).clamp(10.,30.)
        ).color(cor);
        let (negrito, italico) = (e.booleano("bold"), e.booleano("italic"));
        if negrito || italico {
            amostra=amostra.font(iced::Font {
                weight: if negrito { iced::font::Weight::Bold } else { iced::font::Weight::Normal },
                style: if italico { iced::font::Style::Italic } else { iced::font::Style::Normal },
                ..iced::Font::DEFAULT
            });
        }
        let campo = |rotulo: &str, campo: TextoCampo, valor: &str| -> Element<'_, Msg> {
            text_input(rotulo, valor)
                .on_input(move |valor| Msg::FormatoCelula(Evento::Texto(campo, valor)))
                .padding([7, 9]).size(12).width(Length::Fill).into()
        };
        let configuracoes = iced::widget::column![
            Self::formato_secao("Fonte"),
            iced::widget::row![
                container(self.formato_campo("Família", campo("Padrão do tema", TextoCampo::Fonte, &e.fonte)))
                    .width(Length::Fill),
                container(self.formato_campo("Tamanho (6–96)", campo("13", TextoCampo::Tamanho, &e.tamanho)))
                    .width(Length::Fixed(118.)),
            ].spacing(12),
            Self::formato_secao("Estilo"),
            estilos,
            Self::formato_secao("Efeitos"),
            iced::widget::row![
                self.formato_toggle(e,"Sublinhado","underline","underline"),
                self.formato_toggle(e,"Sublinhado duplo","doubleUnderline","underline-double"),
                self.formato_toggle(e,"Tachado","strike","strikethrough"),
                self.formato_toggle(e,"Sobrescrito","superscript","superscript"),
                self.formato_toggle(e,"Subscrito","subscript","subscript"),
            ].spacing(6),
            Self::formato_secao("Prévia"),
            container(amostra)
                .center_x(Length::Fill).center_y(Length::Fixed(92.))
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(COR_PAINEL())),
                    border: iced::Border { color: COR_BORDA_SUAVE(), width: 1.0, radius: 8.0.into() },
                    ..Default::default()
                }),
            text("Sublinhado (simples ou duplo), tachado, sobrescrito e subscrito ficam gravados no estilo da célula.")
                .size(11).color(COR_TEXTO_MUTED()),
        ].spacing(12).width(Length::Fill);
        iced::widget::row![
            container(configuracoes).padding(16)
                .width(Length::Fill).style(estilo_painel_faixa),
            iced::widget::column![
                Self::formato_secao("Cor do texto"),
                super::cores_bordas::seletor_no_dialogo(e,"Cor do texto"),
            ].spacing(10).width(Length::Fixed(278.)),
        ].spacing(16).align_y(Alignment::Start).into()
    }

    fn formato_protecao(&self, e: &Estado) -> Element<'_, Msg> {
        iced::widget::column![
            text("Proteção de célula").size(15),
            self.formato_toggle(e,"Bloqueada","locked","lock"),
            self.formato_toggle(e,"Ocultar fórmula","hidden","eye-off"),
            text("Valem quando a planilha está protegida (menu Formatar, Proteger planilha): a célula bloqueada não muda e a fórmula oculta não aparece na barra.").size(12)
                .color(COR_TEXTO_MUTED()),
        ].spacing(18).into()
    }

    pub(in crate::app) fn view_formatar_celulas(&self) -> Element<'_, Msg> {
        let Some(e)=self.formato_celula.as_ref() else {
            return container(text("Nenhuma seleção de células para formatar")).into();
        };
        let mut abas=iced::widget::row![].spacing(4);
        for (aba,rotulo) in Aba::TODAS {
            abas=abas.push(button(text(rotulo).size(13))
                .style(estilo_botao_pequeno(aba==e.aba))
                .padding([9,14])
                .on_press(Msg::FormatoCelula(Evento::Aba(aba))));
        }
        let conteudo = match e.aba {
            Aba::Numero => self.formato_numero(e),
            Aba::Alinhamento => self.formato_alinhamento(e),
            Aba::Fonte => self.formato_fonte(e),
            Aba::Borda => self.formato_borda(e),
            Aba::Preenchimento => self.formato_preenchimento(e),
            Aba::Protecao => self.formato_protecao(e),
        };
        // A barra de título da janela já informa "Formatar Células".
        // As abas são o primeiro elemento de conteúdo, como no diálogo Excel.
        let rodape=iced::widget::row![
            text(e.erro.clone()).size(11).color(COR_TEXTO_MUTED()).width(Length::Fill),
            button("Cancelar").padding([8,18])
                .style(estilo_botao_secundario)
                .on_press(Msg::FormatoCelula(Evento::Cancelar)),
            button("OK").padding([8,24]).style(estilo_botao_primario)
                .on_press_maybe(e.erro.is_empty().then_some(Msg::FormatoCelula(Evento::Confirmar))),
        ].spacing(9).align_y(Alignment::Center);
        let separador = container(Space::with_height(Length::Fixed(1.)))
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(COR_BORDA())),
                ..Default::default()
            });
        container(iced::widget::column![
            abas,
            separador,
            // Sem rolagem: a janela abre grande o bastante para toda aba (ver `alternar`).
            container(conteudo).padding([8, 1])
                .width(Length::Fill).height(Length::Fill),
            rodape,
        ].spacing(10).padding([12, 14]).height(Length::Fill))
            .width(Length::Fill).height(Length::Fill)
            .style(estilo_container_topo).into()
    }
}
