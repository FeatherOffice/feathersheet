//! Barra de abertura no rodapé: quanto do arquivo já foi lido.
//!
//! Os leitores do `feathersheet-io` somam os bytes em
//! [`feathersheet_io::progresso`]; aqui só se mostra, a cada 100 ms.
use super::*;
use feathersheet_io::progresso::{self, Fase};
use std::sync::atomic::{AtomicU32, Ordering};

/// O percentual que a barra mostra (bits de f32). Persegue o real a cada
/// pulso, para a barra deslizar em vez de pular.
static EXIBIDO: AtomicU32 = AtomicU32::new(0);

/// Um passo da animação; chamado pelo `Msg::ProgressoPulso`.
pub(super) fn animar() {
    let (lidos, total, fase, _) = progresso::agora();
    let alvo = match fase {
        Fase::Lendo if total > 0 => (lidos as f64 / total as f64 * 100.0).min(100.0) as f32,
        Fase::Lendo => 0.0,
        Fase::Montando | Fase::Calculando => 100.0,
    };
    let atual = f32::from_bits(EXIBIDO.load(Ordering::Relaxed));
    // Arquivo novo (o real voltou para trás): recomeça do zero, sem deslizar para trás.
    let novo = if alvo + 1.0 < atual { 0.0 } else { atual + (alvo - atual) * 0.2 };
    EXIBIDO.store(novo.to_bits(), Ordering::Relaxed);
}

impl App {
    pub(super) fn view_progresso_abertura(&self) -> Element<'_, Msg> {
        if !self.carregando {
            return Space::with_width(0).into();
        }
        let (lidos, total, fase, nome) = progresso::agora();
        let mb = |b: u64| b as f64 / 1_048_576.0;
        let (pct, texto) = match fase {
            Fase::Lendo if total > 0 => {
                let pct = (lidos as f64 / total as f64 * 100.0).min(100.0);
                (pct, format!("Abrindo {nome} · {pct:.0}% · {:.0} de {:.0} MB", mb(lidos), mb(total)))
            }
            Fase::Lendo => (0.0, format!("Abrindo {nome}…")),
            Fase::Montando => (100.0, format!("Montando {nome} na memória…")),
            Fase::Calculando => (100.0, format!("Calculando as fórmulas de {nome} em {} núcleos…",
                feathersheet_core::paralelismo::nucleos())),
        };
        let _ = pct;
        let exibido = f32::from_bits(EXIBIDO.load(Ordering::Relaxed));
        row![
            iced::widget::progress_bar(0.0..=100.0, exibido)
                .width(Length::Fixed(150.0)).height(Length::Fixed(6.0))
                .style(|tema| {
                    let mut estilo = iced::widget::progress_bar::primary(tema);
                    estilo.bar = iced::Background::Color(COR_AZUL_ACENTO());
                    estilo.background = iced::Background::Color(COR_BORDA());
                    estilo
                }),
            text(texto).size(11).color(COR_TEXTO_MUTED()),
        ].spacing(8).align_y(Alignment::Center).into()
    }
}

