//! Abas Preenchimento e Borda: controles visuais, prévia e seletor HSV compartilhado.
use super::*;

fn swatch(cor: &str, largura: f32, altura: f32) -> Element<'static, Msg> {
    let cor=if seletor_cor::hex_valido(cor) {
        let [r,g,b,a]=seletor_cor::rgba_de(cor);
        iced::Color::from_rgba8(r,g,b,a as f32/255.)
    } else {iced::Color::TRANSPARENT};
    container(Space::new(Length::Fixed(largura),Length::Fixed(altura)))
        .style(move |_| container::Style {
            background:Some(iced::Background::Color(cor)),
            border:iced::Border {color:COR_BORDA(),width:1.,radius:3.into()},
            ..Default::default()
        }).into()
}

fn borda_ativa(b: &BTreeMap<String, Campo>,lado: &str) -> bool {
    matches!(b.get(lado),Some(Campo::Logico(true)))
}

/// A prévia reflete imediatamente as escolhas do rascunho.
fn desenho_previa(e: &Estado) -> iced::widget::svg::Svg<'static, iced::Theme> {
    let b=e.bordas();
    let rgb=if seletor_cor::hex_valido(&e.cor_borda) {
        e.cor_borda.get(..7).unwrap_or("#808080")
    } else {"#808080"};
    let grossura=match e.texto("borderLineStyle") {
        "Média"=>2.5, "Grossa"=>4.,_=>1.5
    };
    let tracejado=if e.texto("borderLineStyle")=="Tracejada" {
        "stroke-dasharray=\"7 5\"" 
    } else {""};
    let mut linhas=String::new();
    for (lado,geom) in [
        ("top","M18 18H158"), ("right","M158 18V132"),
        ("bottom","M18 132H158"), ("left","M18 18V132")
    ] {
        if borda_ativa(&b,lado) {
            linhas.push_str(&format!(
                "<path d=\"{geom}\" stroke=\"{rgb}\" stroke-width=\"{grossura}\" {tracejado}/>"
            ));
        }
    }
    let svg=format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="176" height="150" viewBox="0 0 176 150" fill="none">
        <rect x="18" y="18" width="140" height="114" fill="#ffffff" fill-opacity="0.25" stroke="#a1aab4" stroke-width="1" stroke-dasharray="4 4"/>
        <path d="M88 18V132 M18 75H158" stroke="#a1aab4" stroke-width="1" stroke-dasharray="3 4"/>
        <text x="88" y="79" fill="#808080" text-anchor="middle" font-size="11" font-family="sans-serif">Texto</text>
        {linhas}
    </svg>"##);
    iced::widget::svg::Svg::new(
        iced::widget::svg::Handle::from_memory(svg.into_bytes())
    ).width(Length::Fixed(176.)).height(Length::Fixed(150.))
}

fn opcao_borda(e: &Estado, lado: &'static str, nome: &'static str) -> Element<'static, Msg> {
    let b=e.bordas();
    let ativo=borda_ativa(&b,lado);
    let icone_nome=match lado {
        "top"=>"Superior","bottom"=>"Inferior","left"=>"Esquerda","right"=>"Direita",_=>"Externas"
    };
    button(iced::widget::row![
        crate::app::menu_bordas::icone_borda(icone_nome,20.),
        text(nome).size(12)
    ].spacing(6).align_y(Alignment::Center))
        .padding([6,9]).style(estilo_botao_pequeno(ativo))
        .on_press(Msg::FormatoCelula(Evento::Borda(lado))).into()
}

pub(in crate::app) fn seletor_no_dialogo(e: &Estado, titulo: &'static str) -> Element<'static, Msg> {
    feather_ui::seletor_cor::painel_embutido(
        titulo, e.cor_atual(), e.cor_hsva, &e.cor_hex_picker
    ).map(|evento| Msg::FormatoCelula(Evento::CorSeletor(evento)))
}

impl App {
    /// Os dois painéis compartilham o mesmo seletor completo do FeatherOffice,
    /// mas editam destinos de estilo diferentes.
    pub(in crate::app) fn formato_borda(&self, e: &Estado) -> Element<'_, Msg> {
        let mut presets=iced::widget::column![].spacing(6);
        // O motor de bordas atual trabalha com quatro lados; não oferecer
        // aqui presets "internas" que simulavam incorretamente bordas externas.
        for opcoes in [["Nenhuma", "Externas", "Todas"]] {
            let mut row=iced::widget::row![].spacing(5);
            for &nome in &opcoes {
                row=row.push(
                    button(iced::widget::row![
                        crate::app::menu_bordas::icone_borda(nome,18.),
                        text(nome).size(11),
                    ].spacing(5).align_y(Alignment::Center))
                    .style(estilo_botao_pequeno(false)).padding([6,8])
                    .on_press(Msg::FormatoCelula(Evento::Borda(nome)))
                );
            }
            presets=presets.push(row);
        }
        let mut estilo_linhas=iced::widget::row![].spacing(7);
        for nome in ["Fina","Média","Grossa","Tracejada"] {
            let selected=e.texto("borderLineStyle")==nome;
            let path=if nome=="Tracejada" {"M3 12H32"} else {"M3 12H32"};
            let size=match nome {"Grossa"=>4.,"Média"=>2.5,_=>1.3};
            let dash=if nome=="Tracejada" {"stroke-dasharray=\"5 3\""} else {""};
            let svg=format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="35" height="24" viewBox="0 0 35 24"><path d="{path}" stroke="#808080" stroke-width="{size}" {dash}/></svg>"##);
            let img=iced::widget::svg::Svg::new(
                iced::widget::svg::Handle::from_memory(svg.into_bytes())
            ).width(Length::Fixed(34.)).height(Length::Fixed(22.));
            estilo_linhas=estilo_linhas.push(
                button(iced::widget::column![img,text(nome).size(11)]
                    .spacing(2).align_x(Alignment::Center))
                    .padding([4,8]).style(estilo_botao_pequeno(selected))
                    .on_press(Msg::FormatoCelula(Evento::Borda(nome)))
            );
        }
        let painel_visual=iced::widget::column![
            text("PRÉVIA DA CÉLULA").size(11).color(COR_TEXTO_MUTED()),
            iced::widget::row![
                Space::with_width(Length::Fixed(87.)),
                opcao_borda(e,"top","Acima"),
            ],
            iced::widget::row![
                opcao_borda(e,"left","Esquerda"),
                desenho_previa(e),
                opcao_borda(e,"right","Direita"),
            ].spacing(5).align_y(Alignment::Center),
            iced::widget::row![
                Space::with_width(Length::Fixed(87.)),
                opcao_borda(e,"bottom","Abaixo"),
            ],
        ].spacing(4);
        let ajustes=iced::widget::column![
            text("PREDEFINIÇÕES").size(11).color(COR_TEXTO_MUTED()),
            presets,
            Space::with_height(Length::Fixed(10.)),
            text("ESTILO DA LINHA").size(11).color(COR_TEXTO_MUTED()),
            estilo_linhas,
            Space::with_height(Length::Fixed(10.)),
            painel_visual,
            text("Clique nos quatro lados para adicionar ou retirar uma borda.")
                .size(11).color(COR_TEXTO_MUTED()),
        ].spacing(6).width(Length::Fill);

        iced::widget::row![
            container(ajustes).padding(12).width(Length::Fill)
                .style(estilo_painel_faixa),
            iced::widget::column![
                text("COR DA BORDA").size(11).color(COR_TEXTO_MUTED()),
                seletor_no_dialogo(e,"Cor da borda"),
            ].spacing(8).width(Length::Fixed(278.)),
        ].spacing(14).align_y(Alignment::Start).into()
    }

    pub(in crate::app) fn formato_preenchimento(&self, e: &Estado) -> Element<'_, Msg> {
        let mut paleta=iced::widget::column![].spacing(6);
        for bloco in CORES.chunks(6) {
            let mut linha=iced::widget::row![].spacing(6);
            for &hex in bloco {
                let cor=seletor_cor::rgba_de(hex);
                let escolhida=e.cor_fundo.eq_ignore_ascii_case(hex);
                let icone=Space::new(Length::Fixed(33.),Length::Fixed(25.));
                linha=linha.push(
                    button(icone).padding(2)
                        .style(move |_,status| {
                            let mut r=estilo_botao_pequeno(escolhida)(
                                &iced::Theme::default(),status);
                            r.background=Some(iced::Background::Color(
                                iced::Color::from_rgb8(cor[0],cor[1],cor[2])));
                            r.border=iced::Border {
                                color:if escolhida {COR_AZUL_ACENTO()} else {COR_BORDA()},
                                width:if escolhida {3.} else {1.},radius:5.into(),
                            };
                            r
                        })
                        .on_press(Msg::FormatoCelula(Evento::CorPredefinida(hex)))
                );
            }
            paleta=paleta.push(linha);
        }
        let cor=if seletor_cor::hex_valido(&e.cor_fundo) {
            let [r,g,b,a]=seletor_cor::rgba_de(&e.cor_fundo);
            iced::Color::from_rgba8(r,g,b,a as f32/255.)
        }else{iced::Color::WHITE};
        let contraste=if (cor.r*0.299+cor.g*0.587+cor.b*0.114)<0.52 {
            iced::Color::WHITE
        }else{iced::Color::from_rgb8(30,33,36)};
        let previa=container(text("Aa   123   •   Prévia do preenchimento")
            .size(14).color(contraste))
            .center_x(Length::Fill).center_y(Length::Fixed(80.))
            .style(move |_|container::Style {
                background:Some(iced::Background::Color(cor)),
                border:iced::Border{color:COR_BORDA(),width:1.,radius:7.into()},
                ..Default::default()
            });
        let painel=iced::widget::column![
            text("PALETA DE CORES").size(11).color(COR_TEXTO_MUTED()),
            paleta,
            Space::with_height(Length::Fixed(14.)),
            text("PRÉ-VISUALIZAÇÃO").size(11).color(COR_TEXTO_MUTED()),
            previa,
            iced::widget::row![
                swatch(&e.cor_fundo,24.,24.),
                text(if e.cor_fundo.is_empty() {"Sem preenchimento".into()}
                    else {e.cor_fundo.clone()}).size(12),
            ].spacing(9).align_y(Alignment::Center),
            Space::with_height(Length::Fixed(6.)),
            button(iced::widget::row![
                feather_ui::icones::icone("eraser",15.),
                text("Remover preenchimento").size(12)
            ].spacing(8).align_y(Alignment::Center))
                .padding([8,12]).style(estilo_botao_secundario)
                .on_press(Msg::FormatoCelula(Evento::Texto(
                    TextoCampo::CorFundo,String::new()))),
        ].spacing(8).width(Length::Fill);
        iced::widget::row![
            container(painel).padding(14).width(Length::Fill)
                .style(estilo_painel_faixa),
            iced::widget::column![
                text("ESCOLHER COR").size(11).color(COR_TEXTO_MUTED()),
                seletor_no_dialogo(e,"Preenchimento"),
            ].spacing(8).width(Length::Fixed(278.)),
        ].spacing(14).align_y(Alignment::Start).into()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn abas_exibem_seletor_hsv_e_previa_de_borda() {
        let mut app=App::default();
        app.formato_celula=Some(Estado::novo(&app,Aba::Borda));
        let e=app.formato_celula.as_ref().unwrap();
        let _: Element<'_,Msg>=app.formato_borda(e);
        let _: Element<'_,Msg>=app.formato_preenchimento(e);
        let svg=desenho_previa(e);
        let _: Element<'_,Msg>=svg.into();
    }
}
