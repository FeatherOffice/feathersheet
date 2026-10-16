//! Integração nativa dos gestos de preenchimento do antigo `preencher.ts`.
//! A engine de séries/fórmulas continua no `feathersheet-core::operacoes::fill`.
//! A interface calcula somente fonte/destino e impede escritas ambíguas.
use super::*;

/// O motor atual prepara as células numa Vec antes de aplicar a transação.
/// Evitar uma alocação gigantesca e bloqueio da interface até existir lotes.
const MAX_CELULAS_POR_ACAO: u64 = 250_000;

fn tamanho(area: &Rect) -> u64 {
    (u64::from(area.r1) - u64::from(area.r0) + 1)
        * (u64::from(area.c1) - u64::from(area.c0) + 1)
}

/// A primeira linha/coluna é o padrão a copiar, como Ctrl+D/Ctrl+R do Excel.
/// Para cima/esquerda, usa a última linha/coluna e extrapola em sentido inverso.
fn origem_direcional(
    sel: &Rect, direcao: DirecaoPreenchimento,
) -> Result<Rect, &'static str> {
    let sel = sel.normalized();
    match direcao {
        DirecaoPreenchimento::Baixo if sel.r0 < sel.r1 =>
            Ok(Rect::new(sel.r0, sel.c0, sel.r0, sel.c1)),
        DirecaoPreenchimento::Cima if sel.r0 < sel.r1 =>
            Ok(Rect::new(sel.r1, sel.c0, sel.r1, sel.c1)),
        DirecaoPreenchimento::Direita if sel.c0 < sel.c1 =>
            Ok(Rect::new(sel.r0, sel.c0, sel.r1, sel.c0)),
        DirecaoPreenchimento::Esquerda if sel.c0 < sel.c1 =>
            Ok(Rect::new(sel.r0, sel.c1, sel.r1, sel.c1)),
        DirecaoPreenchimento::Baixo | DirecaoPreenchimento::Cima =>
            Err(tr!("Selecione duas ou mais linhas para preencher nesta direção.")),
        DirecaoPreenchimento::Direita | DirecaoPreenchimento::Esquerda =>
            Err(tr!("Selecione duas ou mais colunas para preencher nesta direção.")),
        DirecaoPreenchimento::AteOFim =>
            Err(tr!("Continuar até o fim requer detectar a coluna de dados vizinha.")),
    }
}

impl App {
    pub(super) fn preencher_selecao(&mut self, direcao: DirecaoPreenchimento) {
        self.edicao_menu = None;
        if !self.confirmar() { return; }
        let selecao = self.area_selecionada().normalized();
        let (origem, destino) = if direcao == DirecaoPreenchimento::AteOFim {
            // Chama o localizador do antigo duplo clique, sem escrever ainda.
            let previsto = self.workbook.aba_mut(self.folha)
                .and_then(|aba| aba.sheet.preencher_ate_o_fim(&selecao, MAX_COLS, true))
                .map(|(destino, _)| destino);
            let Some(destino) = previsto else {
                self.status = tr!("Nenhuma continuação encontrada na coluna vizinha.").into();
                return;
            };
            (selecao.clone(), destino)
        } else {
            let origem = match origem_direcional(&selecao, direcao) {
                Ok(origem) => origem,
                Err(msg) => { self.status = msg.into(); return; }
            };
            (origem, selecao.clone())
        };
        self.aplicar_preenchimento(&origem, &destino, direcao == DirecaoPreenchimento::AteOFim);
    }

    /// A alça de preenchimento arrastada: a seleção é a origem e `destino`
    /// é o retângulo que a prévia tracejada mostrava (já estendido num eixo
    /// por `fill_target`). No fim a seleção passa a ser o destino, como no Excel.
    pub(super) fn preencher_pela_alca(&mut self, destino: Rect) {
        self.edicao_menu = None;
        if !self.confirmar() { return; }
        let origem = self.area_selecionada().normalized();
        let destino = destino.normalized();
        if destino == origem { return; }
        self.aplicar_preenchimento(&origem, &destino, true);
    }

    /// Validações comuns e a escrita (um Ctrl+Z). Com `selecionar_destino`,
    /// a seleção cobre o destino inteiro depois de preencher.
    pub(super) fn aplicar_preenchimento(&mut self, origem: &Rect, destino: &Rect, selecionar_destino: bool) {
        let (origem, destino) = (origem.clone(), destino.clone());
        let quantidade = tamanho(&destino).saturating_sub(tamanho(&origem));
        if quantidade == 0 {
            self.status = tr!("Não há células de destino para preencher.").into();
            return;
        }
        if quantidade > MAX_CELULAS_POR_ACAO {
            self.status = trf!(
                "Preenchimento de {quantidade} células excede o limite de \
                 {MAX_CELULAS_POR_ACAO} células por operação síncrona; selecione um intervalo menor."
            );
            return;
        }
        let Some(sheet) = self.workbook.aba(self.folha).map(|a| &a.sheet) else { return };
        // Não sobrepor mesclas nem preencher a partir de um bloco sem valores.
        let mesclas = mesclas::ler(sheet);
        if mesclas.iter().any(|area| mesclas::cruzam(area, &destino)) {
            self.status = tr!("Preenchimento bloqueado: o destino cruza células mescladas.").into();
            return;
        }
        let tem_origem = (origem.r0..=origem.r1).any(|r| {
            (origem.c0..=origem.c1).any(|c| !sheet.get_raw(r, c).is_empty())
        });
        if !tem_origem {
            self.status = tr!("A origem está vazia; escolha células com dados ou fórmulas.").into();
            return;
        }
        let mut preenchidas = 0usize;
        if self.workbook.editar(self.folha, |aba| {
            preenchidas = aba.preencher(&origem, &destino);
        }).is_none() {
            self.status = tr!("A planilha não está disponível para preenchimento.").into();
            return;
        }
        if preenchidas > 0 {
            self.alterado = true;
            // Em continuar até o fim (e na alça), incluir o destino na seleção.
            if selecionar_destino {
                self.linha = destino.r0;
                self.coluna = destino.c0;
                self.fim_selecao = (destino.r1, destino.c1);
            }
            self.status = trf!(
                "{preenchidas} célula(s) preenchida(s) em {}:{}; Ctrl+Z desfaz.",
                cell_name(destino.r0, destino.c0), cell_name(destino.r1, destino.c1)
            );
        } else {
            self.status = tr!("Nenhuma célula foi preenchida.").into();
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathersheet_core::celulas::values::Value;

    fn selecionar(app: &mut App, r0: u32, c0: u32, r1: u32, c1: u32) {
        app.linha = r0;
        app.coluna = c0;
        app.fim_selecao = (r1, c1);
    }

    #[test]
    fn ctrl_d_copia_formula_relativa_e_desfaz_em_uma_etapa() {
        let mut app = App::default();
        app.workbook.escrever_celula(0, 0, 0, "2".into());
        app.workbook.escrever_celula(0, 0, 1, "=A1*3".into());
        selecionar(&mut app, 0, 1, 3, 1);
        app.preencher_selecao(DirecaoPreenchimento::Baixo);
        let s = &app.workbook.aba(0).unwrap().sheet;
        assert_eq!(s.get_raw(3, 1), "=A4*3");
        assert!(app.alterado);
        app.historico(false);
        assert!(app.workbook.aba(0).unwrap().sheet.get_raw(3, 1).is_empty());
    }

    #[test]
    fn alca_continua_a_serie_e_seleciona_o_destino() {
        let mut app = App::default();
        app.workbook.escrever_celula(0, 0, 0, "1".into());
        app.workbook.escrever_celula(0, 1, 0, "2".into());
        selecionar(&mut app, 0, 0, 1, 0);
        app.preencher_pela_alca(Rect::new(0, 0, 4, 0));
        assert_eq!(app.workbook.valor(0, 4, 0), Value::Number(5.0));
        assert_eq!((app.linha, app.coluna, app.fim_selecao), (0, 0, (4, 0)));
        // Um Ctrl+Z desfaz o arrasto inteiro.
        app.historico(false);
        assert_eq!(app.workbook.valor(0, 2, 0), Value::Empty);
    }

    #[test]
    fn preenche_direita_e_esquerda_sem_mudar_celulas_fora_da_selecao() {
        let mut app = App::default();
        app.workbook.escrever_celula(0, 0, 0, "x".into());
        app.workbook.escrever_celula(0, 1, 2, "fim".into());
        selecionar(&mut app, 0, 0, 0, 1);
        app.preencher_selecao(DirecaoPreenchimento::Direita);
        assert_eq!(app.workbook.valor(0, 0, 1), Value::Text("x".into()));
        selecionar(&mut app, 1, 1, 1, 2);
        app.preencher_selecao(DirecaoPreenchimento::Esquerda);
        assert_eq!(app.workbook.valor(0, 1, 1), Value::Text("fim".into()));
        assert_eq!(app.workbook.valor(0, 0, 2), Value::Empty);
    }

    #[test]
    fn preencher_para_cima_ajusta_formula_relativa() {
        let mut app = App::default();
        app.workbook.escrever_celula(0, 2, 1, "=A3*2".into());
        selecionar(&mut app, 0, 1, 2, 1);
        app.preencher_selecao(DirecaoPreenchimento::Cima);
        let sheet = &app.workbook.aba(0).unwrap().sheet;
        assert_eq!(sheet.get_raw(0, 1), "=A1*2");
        assert_eq!(sheet.get_raw(1, 1), "=A2*2");
        assert_eq!(sheet.get_raw(2, 1), "=A3*2");
    }

    #[test]
    fn bloqueia_grande_previa_antes_de_alocar_plano() {
        let mut app = App::default();
        app.workbook.escrever_celula(0, 0, 0, "valor".into());
        selecionar(&mut app, 0, 0, 300_000, 0);
        app.preencher_selecao(DirecaoPreenchimento::Baixo);
        assert!(app.status.contains("limite"));
        assert!(!app.alterado);
        assert!(app.workbook.aba(0).unwrap().sheet.get_raw(300_000, 0).is_empty());
    }

    #[test]
    fn continuar_serie_ate_fim_de_coluna_vizinha() {
        let mut app = App::default();
        for r in 0..6 {
            app.workbook.escrever_celula(0, r, 0, format!("{}", r + 1));
        }
        app.workbook.escrever_celula(0, 0, 1, "10".into());
        app.workbook.escrever_celula(0, 1, 1, "20".into());
        selecionar(&mut app, 0, 1, 1, 1);
        app.preencher_selecao(DirecaoPreenchimento::AteOFim);
        assert_eq!(app.workbook.valor(0, 5, 1), Value::Number(60.0));
        assert_eq!(app.fim_selecao, (5, 1));
    }

    #[test]
    fn nunca_limpa_destino_por_origem_vazia_ou_mescla() {
        let mut app = App::default();
        app.workbook.escrever_celula(0, 2, 0, "manter".into());
        selecionar(&mut app, 1, 0, 2, 0);
        app.preencher_selecao(DirecaoPreenchimento::Baixo);
        assert_eq!(app.workbook.valor(0, 2, 0), Value::Text("manter".into()));
        app.workbook.escrever_celula(0, 1, 0, "origem".into());
        app.workbook.editar(0, |s| {
            mesclas::mesclar(s, Rect::new(2, 0, 2, 1));
        });
        app.preencher_selecao(DirecaoPreenchimento::Baixo);
        assert_eq!(app.workbook.valor(0, 2, 0), Value::Text("manter".into()));
    }
}
