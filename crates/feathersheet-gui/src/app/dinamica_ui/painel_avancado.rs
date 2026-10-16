//! Controles avançados da tabela dinâmica nativa (filtros, layout e calculados).
use super::*;
use iced::widget::{column, pick_list};
use feathersheet_core::dinamica::{campo_calculado, dinamica_layout::Layout};

impl App {
    pub(super) fn adicionar_campo_calculado(&mut self) {
        let Some(e) = self.dinamica.as_mut() else { return };
        let nome = e.novo_nome.trim().to_owned();
        let formula = e.nova_formula.trim().to_owned();
        if nome.is_empty() || formula.is_empty() {
            self.status = "Informe nome e fórmula para o campo calculado.".into();
            return;
        }
        if e.nomes.iter().any(|existente| existente.eq_ignore_ascii_case(&nome)) {
            self.status = format!("O campo '{nome}' já existe na tabela dinâmica.");
            return;
        }
        let real = e.nomes.len().saturating_sub(e.calculados.len());
        if (e.origem.c1 as usize) + e.calculados.len() + 1 >= MAX_COLS as usize {
            self.status = "Não existem mais colunas disponíveis para campos calculados.".into();
            return;
        }
        if let Err(msg) = campo_calculado::compilar(&formula, &e.nomes[..real]) {
            self.status = format!("Fórmula inválida: {msg}");
            return;
        }
        let col = e.origem.c1 as usize + 1 + e.calculados.len();
        e.calculados.push(Calculado { nome: nome.clone(), formula });
        e.nomes.push(nome);
        e.valores.push(MedidaDinamica { coluna: col, op: AgregacaoOp::Soma });
        e.novo_nome.clear();
        e.nova_formula.clear();
        e.previa_calculado = None;
        self.status = "Campo calculado adicionado. Clique em Criar/Aplicar para atualizar o resultado.".into();
    }

    pub(super) fn remover_campo_calculado(&mut self, indice: usize) {
        let Some(e) = self.dinamica.as_mut() else { return };
        if indice >= e.calculados.len() { return; }
        let col = e.origem.c1 as usize + 1 + indice;
        let real = e.nomes.len().saturating_sub(e.calculados.len());
        e.calculados.remove(indice);
        e.nomes.remove(real + indice);
        let atualizar = |a: &mut Vec<usize>| {
            a.retain(|&c| c != col);
            for c in a { if *c > col { *c -= 1; } }
        };
        atualizar(&mut e.linhas);
        atualizar(&mut e.colunas);
        e.valores.retain(|v| v.coluna != col);
        for v in &mut e.valores { if v.coluna > col { v.coluna -= 1; } }
        e.filtros.retain(|f| f.coluna != col);
        for f in &mut e.filtros { if f.coluna > col { f.coluna -= 1; } }
        e.campo_filtro = None;
        e.valores_filtro.clear();
    }

    /// Opções recolhíveis do painel: uma por linha, rótulo à esquerda e
    /// controle à direita (antes eram uma fileira só que estourava a largura).
    pub(super) fn controles_avancados<'a>(&'a self, e: &'a Estado) -> Element<'a, Msg> {
        let linha = |rotulo: &'static str, controle: Element<'a, Msg>| -> Element<'a, Msg> {
            row![text(rotulo).size(12).width(Length::Fill), controle]
                .spacing(8).align_y(Alignment::Center).into()
        };
        let segmento = |rotulo: &'static str, ativo: bool, msg: Msg| -> Element<'a, Msg> {
            button(text(rotulo).size(11.5)).padding([4, 9])
                .style(estilo_botao_pequeno(ativo)).on_press(msg).into()
        };
        let tabular = e.layout == Layout::Tabular;
        const AGRUPAR: [Option<AgruparData>; 4] =
            [None, Some(AgruparData::Mes), Some(AgruparData::Trimestre), Some(AgruparData::Ano)];
        let rotulos_agrupar: Vec<&'static str> = AGRUPAR.iter().map(|m| rotulo_agrupamento(*m)).collect();
        let mut grupos = column![].spacing(2);
        for caminho in &e.grupos {
            let fechado = e.recolhidos.contains(caminho);
            grupos = grupos.push(button(row![
                feather_ui::icones::icone(if fechado { "chevron-right" } else { "chevron-down" }, 12.0),
                text(caminho.split(feathersheet_core::dinamica::dinamica_layout::SEPARADOR)
                    .collect::<Vec<_>>().join(" / ")).size(11),
            ].spacing(5).align_y(Alignment::Center))
                .width(Length::Fill).style(estilo_botao_pequeno(fechado))
                .on_press(Msg::Dinamica(Evento::Recolher(caminho.clone()))));
        }
        let painel_grupos: Element<'a, Msg> = if e.grupos.is_empty() {
            text("Os grupos aparecem depois que a tabela tiver linhas agrupadas.")
                .size(11).color(COR_TEXTO_MUTED()).into()
        } else {
            column![
                row![
                    button(text("Recolher tudo").size(11)).style(estilo_botao_secundario)
                        .on_press(Msg::Dinamica(Evento::RecolherTodos(true))),
                    button(text("Expandir tudo").size(11)).style(estilo_botao_secundario)
                        .on_press(Msg::Dinamica(Evento::RecolherTodos(false))),
                ].spacing(6),
                scrollable(grupos).height(Length::Fixed(90.0)),
            ].spacing(6).into()
        };
        let mut calculados = column![].spacing(3);
        for (i, calculado) in e.calculados.iter().enumerate() {
            calculados = calculados.push(row![
                text(format!("{} = {}", calculado.nome, calculado.formula)).size(11).width(Length::Fill),
                button(feather_ui::icones::icone("trash", 13.0)).padding([3, 5])
                    .style(estilo_botao_pequeno(false))
                    .on_press(Msg::Dinamica(Evento::RemoverCalculado(i))),
            ].spacing(6).align_y(Alignment::Center));
        }
        let previa: Element<'a, Msg> = if let Some(valores) = &e.previa_calculado {
            text(format!("Prévia: {}", valores.iter().map(|v| v.to_string())
                .collect::<Vec<_>>().join(" · "))).size(11).color(COR_TEXTO_MUTED()).into()
        } else { Space::with_height(0).into() };
        let titulo = |t: &'static str| text(t).size(10.5).color(COR_TEXTO_MUTED());
        column![
            titulo("LAYOUT"),
            linha("Formato do relatório", row![
                segmento("Compacto", !tabular, Msg::Dinamica(Evento::LayoutEscolher(false))),
                segmento("Tabular", tabular, Msg::Dinamica(Evento::LayoutEscolher(true))),
            ].spacing(4).into()),
            linha("Repetir rótulos", toggler(e.repetir_rotulos)
                .on_toggle(|_| Msg::Dinamica(Evento::RepetirRotulos)).style(estilo_toggle).into()),
            linha("Subtotais", toggler(e.subtotais)
                .on_toggle(|_| Msg::Dinamica(Evento::Subtotais)).style(estilo_toggle).into()),
            linha("Agrupar datas", pick_list(rotulos_agrupar, Some(rotulo_agrupamento(e.agrupar_data)),
                |escolha| Msg::Dinamica(Evento::AgruparEscolher(
                    AGRUPAR.into_iter().find(|m| rotulo_agrupamento(*m) == escolha).flatten())))
                .text_size(11.5).padding([4, 8]).into()),
            linha("Cores", pick_list(TEMAS_DINAMICA, TEMAS_DINAMICA.iter().copied().find(|t| *t == e.tema),
                |t| Msg::Dinamica(Evento::TemaEscolher(t)))
                .text_size(11.5).padding([4, 8]).into()),
            titulo("ORIGEM DOS DADOS"),
            row![
                text_input("A1:D100", &e.origem_texto)
                    .on_input(|v| Msg::Dinamica(Evento::OrigemTexto(v)))
                    .on_submit(Msg::Dinamica(Evento::OrigemAplicar)).padding(5).size(12),
                button(text("Aplicar").size(11.5)).style(estilo_botao_secundario)
                    .on_press(Msg::Dinamica(Evento::OrigemAplicar)),
            ].spacing(6).align_y(Alignment::Center),
            titulo("GRUPOS"),
            painel_grupos,
            titulo("CAMPOS CALCULADOS"),
            calculados,
            text_input("Nome do campo", &e.novo_nome)
                .on_input(|s| Msg::Dinamica(Evento::NomeCalculado(s))).padding(6).size(12),
            text_input("Fórmula: [Preço] * Quantidade", &e.nova_formula)
                .on_input(|s| Msg::Dinamica(Evento::FormulaCalculado(s))).padding(6).size(12),
            row![
                Space::with_width(Length::Fill),
                button(text("Prévia").size(11.5)).style(estilo_botao_secundario)
                    .on_press(Msg::Dinamica(Evento::PreviaCalculado)),
                button(text("Adicionar").size(11.5)).style(estilo_botao_primario)
                    .on_press(Msg::Dinamica(Evento::AdicionarCalculado)),
            ].spacing(6),
            previa,
        ].spacing(9).into()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn previa_de_campo_calculado_nao_altera_a_origem() {
        let mut app = App::default();
        app.workbook.editar(0, |s| {
            s.set_cell(0,0,"Preço"); s.set_cell(0,1,"Qtd");
            s.set_cell(1,0,"10"); s.set_cell(1,1,"2");
            s.set_cell(2,0,"20"); s.set_cell(2,1,"3");
        });
        app.fim_selecao = (2,1);
        app.evento_dinamica(Evento::Abrir);
        app.evento_dinamica(Evento::FormulaCalculado("Preço * Qtd".into()));
        app.evento_dinamica(Evento::PreviaCalculado);
        assert_eq!(app.dinamica.as_ref().unwrap().previa_calculado,Some(vec![20.,60.]));
        assert_eq!(app.workbook.valor(0,2,1),Value::Number(3.));
        assert_eq!(app.workbook.len(),1);
    }

    #[test]
    fn campo_calculado_e_removido_sem_indices_pendentes() {
        let mut app = App::default();
        app.workbook.editar(0, |s| {
            s.set_cell(0,0,"Região"); s.set_cell(0,1,"Preço"); s.set_cell(0,2,"Quantidade");
            s.set_cell(1,0,"Sul"); s.set_cell(1,1,"10"); s.set_cell(1,2,"2");
        });
        app.fim_selecao = (1,2);
        app.evento_dinamica(Evento::Abrir);
        app.evento_dinamica(Evento::NomeCalculado("Receita".into()));
        app.evento_dinamica(Evento::FormulaCalculado("[Preço] * Quantidade".into()));
        app.evento_dinamica(Evento::AdicionarCalculado);
        let e = app.dinamica.as_ref().expect("janela aberta");
        assert_eq!(e.calculados.len(), 1);
        assert_eq!(e.valores.last().unwrap().coluna, 3);
        app.evento_dinamica(Evento::RemoverCalculado(0));
        let e = app.dinamica.as_ref().unwrap();
        assert!(e.calculados.is_empty());
        assert!(!e.valores.iter().any(|v| v.coluna == 3));
    }
}
