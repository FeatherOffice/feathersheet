//! Substituição de arquivos sem expor conteúdo parcialmente escrito.
use std::{fs::{self, OpenOptions}, io::{self, Write}, path::Path, sync::atomic::{AtomicU64, Ordering}};

static SEQUENCIA: AtomicU64 = AtomicU64::new(0);

/// O temporário fica no mesmo diretório para que o rename seja atômico.
/// Uma falha antes do rename preserva o arquivo anterior.
pub fn gravar_atomico(destino: &Path, bytes: &[u8]) -> io::Result<()> {
    gravar_atomico_com(destino, |arquivo| arquivo.write_all(bytes))
}

/// Salva em streaming, sem criar um buffer com o arquivo todo.
/// Se a escrita falhar, o temporário é excluído e o destino é preservado.
pub fn gravar_atomico_com(
    destino: &Path,
    escrever: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> io::Result<()> {
    let nome = destino.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Arquivo sem nome"))?;
    let mut tentativa = 0;
    let (temporario, mut arquivo) = loop {
        let mut nome_temporario = nome.to_os_string();
        nome_temporario.push(format!(".{}.{}.parcial", std::process::id(), SEQUENCIA.fetch_add(1, Ordering::Relaxed)));
        let temporario = destino.with_file_name(nome_temporario);
        match OpenOptions::new().write(true).create_new(true).open(&temporario) {
            Ok(arquivo) => break (temporario, arquivo),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists && tentativa < 16 => tentativa += 1,
            Err(e) => return Err(e),
        }
    };
    let resultado = (|| {
        if let Ok(meta) = fs::metadata(destino) { arquivo.set_permissions(meta.permissions())?; }
        escrever(&mut arquivo)?;
        arquivo.sync_all()?;
        drop(arquivo);
        fs::rename(&temporario, destino)
    })();
    if resultado.is_err() { let _ = fs::remove_file(&temporario); }
    resultado
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn substitui_e_remove_temporario_em_falha() {
        let dir = std::env::temp_dir().join(format!("feathersheet-io-{}-{}", std::process::id(), SEQUENCIA.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&dir).unwrap();
        let destino = dir.join("planilha.fshz");
        gravar_atomico(&destino, b"antes").unwrap();
        gravar_atomico(&destino, b"depois").unwrap();
        assert_eq!(fs::read(&destino).unwrap(), b"depois");
        assert!(gravar_atomico_com(&destino, |arquivo| {
            arquivo.write_all(b"incompleto")?;
            Err(io::Error::other("falha simulada do compressor"))
        }).is_err());
        assert_eq!(fs::read(&destino).unwrap(), b"depois");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "remove temporário após falha");
        let pasta = dir.join("pasta");
        fs::create_dir(&pasta).unwrap();
        assert!(gravar_atomico(&pasta, b"erro").is_err());
        assert!(pasta.is_dir());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        fs::remove_file(destino).unwrap();
        fs::remove_dir(pasta).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn falha_do_exportador_em_stream_preserva_destino() {
        let dir = std::env::temp_dir().join(format!(
            "feathersheet-stream-{}-{}",
            std::process::id(), SEQUENCIA.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&dir).unwrap();
        let destino = dir.join("dados.csv");
        gravar_atomico(&destino, b"original").unwrap();
        let falha = gravar_atomico_com(&destino, |arquivo| {
            arquivo.write_all(b"incompleto")?;
            Err(io::Error::new(io::ErrorKind::Other, "escrita interrompida"))
        });
        assert!(falha.is_err());
        assert_eq!(fs::read(&destino).unwrap(), b"original");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_file(destino).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
