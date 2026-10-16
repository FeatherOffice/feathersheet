//! "O que há de novo": abre sozinha na primeira abertura depois de atualizar
//! (e em Ajuda ▸ O que há de novo). Segue a F1 (GUIA-JANELAS.md): lateral com
//! as versões, cabeçalho com a marca, cartões dos destaques e a lista do
//! resto. O conteúdo é embutido no app; nada é buscado na internet.
use super::*;
use feather_ui::consulta::{self, Ilustracao};

/// Uma versão publicada e o que ela trouxe.
pub struct Versao {
    pub numero: &'static str,
    pub data: &'static str,
    pub resumo: &'static str,
    /// Figura do cabeçalho da versão.
    pub ilustracao: Ilustracao,
    /// Cartões: ícone, título, descrição curta (até ~84 caracteres).
    pub destaques: &'static [(&'static str, &'static str, &'static str)],
    /// "Também nesta versão": melhorias menores e correções.
    pub tambem: &'static [&'static str],
}

/// Da mais nova para a mais antiga. A primeira é a desta compilação.
pub const VERSOES: &[Versao] = &[
    Versao {
        numero: "0.0.4",
        data: "16/10/2026",
        resumo: "Novidades ao atualizar",
        ilustracao: Ilustracao::Atualizacao,
        destaques: &[
            ("sparkles", "O que há de novo ao atualizar", "Quem vem de uma versão anterior vê esta janela na primeira abertura."),
            ("rocket", "Primeiros passos na instalação", "O assistente abre sozinho só numa instalação nova; depois, no menu Ajuda."),
        ],
        tambem: &[
            "Na 0.0.3, quem atualizava da 0.0.2 recebia os Primeiros passos em vez das novidades.",
        ],
    },
    Versao {
        numero: "0.0.3",
        data: "16/10/2026",
        resumo: "Gráficos no visual novo e recuperação automática",
        ilustracao: Ilustracao::Grafico,
        destaques: &[
            ("chart-bar", "25 tipos de gráfico", "Todos no visual novo, inclusive combinado, cascata, radar, Gantt e mapa de calor."),
            ("pointer", "Dica do mouse", "Passe o mouse num ponto para ver o valor; o texto da dica é personalizável."),
            ("list", "Legenda clicável", "Clique numa série da legenda para ocultá-la, com transição suave."),
            ("chart-line", "Elementos analíticos", "Linha de tendência, barras de erro, máximo, mínimo e média."),
            ("history", "Recuperação automática", "Se o computador desligar, a planilha alterada volta pela tela inicial."),
            ("arrow-back-up", "Ctrl+Z nos gráficos", "Mover e redimensionar um gráfico agora se desfaz como qualquer edição."),
            ("rocket", "Primeiros passos", "Assistente da primeira abertura e esta janela de novidades."),
            ("user", "Seus dados", "Seu nome assina os comentários e vai nas propriedades do .fshz e do .xlsx."),
        ],
        tambem: &[
            "Shift + roda do mouse rola a planilha na horizontal.",
            "Painel de formatação do gráfico com grupos separados por divisórias.",
            "Gráficos importados do .xlsx sem preenchimento voltam a ter fundo.",
            "A barra de abertura do .parquet conta linhas, não megabytes.",
            "O rodapé (Contagem, Soma) não para mais no meio da seleção em pastas novas.",
        ],
    },
    Versao {
        numero: "0.0.2",
        data: "16/10/2026",
        resumo: "Atualização pelo app e grade mais leve",
        ilustracao: Ilustracao::Atualizacao,
        destaques: &[
            ("refresh", "Procurar atualização", "Em Configurações, Sobre: consulta a versão nova e instala à vista, num terminal."),
            ("table", "Grade mais leve", "O desenho da grade fica guardado e só é refeito quando algo muda."),
            ("device-desktop", "Nome e logo no sistema", "O monitor do sistema do Linux mostra o FeatherSheet com nome e ícone."),
        ],
        tambem: &[
            "O cursor só é acompanhado na janela principal, sem lentidão com outra janela aberta.",
            "Identificação da janela e ícone corrigidos.",
        ],
    },
    Versao {
        numero: "0.0.1",
        data: "15/10/2026",
        resumo: "Primeira versão pública",
        ilustracao: Ilustracao::Planilha,
        destaques: &[
            ("table", "Planilha nativa", "Motor e interface em Rust, rápidos com arquivos grandes."),
            ("math-function", "Funções de planilha", "Nomes em português e em inglês, com ajuda e exemplos na F1."),
            ("code", "Macros", "Editor de macros em JavaScript (Alt+F11) e importação de VBA."),
            ("device-floppy", "Instaladores", "Pacote .deb, .tar.gz e instalador do Windows."),
        ],
        tambem: &[
            "Repositório próprio, com a paridade com o Excel descrita no README.",
        ],
    },
];

#[derive(Debug, Clone)]
pub enum Evento {
    Versao(usize),
    VerNoGithub,
    Fechar,
}

impl App {
    pub(super) fn evento_novidades(&mut self, evento: Evento) -> Task<Msg> {
        match evento {
            Evento::Versao(i) => self.novidades_versao = i.min(VERSOES.len() - 1),
            Evento::VerNoGithub => {
                let v = &VERSOES[self.novidades_versao.min(VERSOES.len() - 1)];
                if !cfg!(test) {
                    super::atualizacao::abrir_endereco(&format!("{}/tag/v{}", super::atualizacao::PAGINA_VERSOES, v.numero));
                }
            }
            Evento::Fechar => {
                if let Some(id) = self.janelas.id(Auxiliar::Novidades) {
                    return self.update(Msg::Fechar(id));
                }
            }
        }
        Task::none()
    }

    pub(super) fn view_novidades(&self) -> Element<'_, Msg> {
        use iced::widget::column;
        let atual = self.novidades_versao.min(VERSOES.len() - 1);
        let mut nav = column![consulta::rotulo_grupo("Versões")].spacing(2);
        for (i, v) in VERSOES.iter().enumerate() {
            nav = nav.push(consulta::item_nav(format!("Versão {}", v.numero),
                if i == 0 { "sparkles" } else { "history" }, i == atual,
                Some(Msg::Novidades(Evento::Versao(i)))));
        }
        let v = &VERSOES[atual];
        let instalada = v.numero == super::atualizacao::versao_atual();
        let mut tambem = column![].spacing(8);
        for item in v.tambem {
            tambem = tambem.push(row![
                feather_ui::icones::icone("check", 15.),
                text(*item).size(13).line_height(1.4),
            ].spacing(9).align_y(Alignment::Start));
        }
        let conteudo = column![
            consulta::cabecalho(format!("Versão {}", v.numero),
                format!("{} · {}{}", v.data, v.resumo, if instalada { " · instalada neste computador" } else { "" }),
                Some(consulta::ilustracao(v.ilustracao))),
            consulta::negrito("Destaques", 18.),
            consulta::grade(v.destaques.iter()
                .map(|(icone, titulo, descricao)| consulta::cartao(icone, *titulo, *descricao, None))
                .collect(), 3),
            consulta::divisor(),
            consulta::negrito("Também nesta versão", 18.),
            tambem,
        ].spacing(17);
        let rodape = row![
            Space::with_width(Length::Fill),
            button(row![feather_ui::icones::icone("link", 15.), text("Ver no GitHub").size(12)]
                .spacing(6).align_y(Alignment::Center))
                .padding([8, 13]).style(consulta::estilo_cartao)
                .on_press(Msg::Novidades(Evento::VerNoGithub)),
            button(text("OK").size(12)).padding([8, 18]).style(estilo_botao_primario)
                .on_press(Msg::Novidades(Evento::Fechar)),
        ].spacing(10).align_y(Alignment::Center);
        consulta::janela(consulta::lateral(nav, None), conteudo.into(), Some(rodape.into()))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn versao_desta_compilacao_e_a_primeira_e_todas_desenham() {
        assert_eq!(VERSOES[0].numero, super::super::atualizacao::versao_atual(),
            "acrescente as novidades da versão nova em VERSOES");
        let mut app = App::default();
        for i in 0..VERSOES.len() + 1 {
            let _ = app.update(Msg::Novidades(Evento::Versao(i)));
            let _: Element<'_, Msg> = app.view_novidades();
        }
        assert_eq!(app.novidades_versao, VERSOES.len() - 1);
        for v in VERSOES {
            for (icone, _, descricao) in v.destaques {
                assert!(feather_ui::icones::existe(icone), "{icone}");
                assert!(descricao.chars().count() <= 90, "{descricao}");
            }
        }
    }
}
