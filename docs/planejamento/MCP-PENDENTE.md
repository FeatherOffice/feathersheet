# MCP do FeatherSheet — o que falta

> Levantamento de 14/10/2026, logo depois da primeira versão (`d177e8f5`, `a4a15506`).
> Crate: `crates/feathersheet-mcp`; ponte ao vivo: `crates/feathersheet-gui/src/app/mcp_vivo.rs`.
> Regra que vale para tudo abaixo: **nunca despejar a planilha** — o motor calcula, a IA recebe
> resumo, amostra ou resultado; cada chamada que escreve é **um Ctrl+Z**.

## O que já existe

| Ferramenta | Arquivo (sem o app) | Ao vivo |
|---|---|---|
| `sheet_resumo`, `sheet_ler` (teto 2000), `sheet_amostra`, `sheet_estatistica`, `sheet_localizar` | sim | sim |
| `sheet_escrever` (valores, fórmula pt-BR, preencher fórmula), `sheet_limpar`, `sheet_desfazer` | sim | sim |
| `sheet_aba`, `sheet_nomes`, `sheet_formula` (precedentes, dependentes, erro, passo a passo) | sim | sim |
| `sheet_consulta` (passos do OpenQuery, grava em aba), `sheet_macro` (catálogo comum + executar) | sim | sim |
| `sheet_formatar`, `sheet_estrutura`, `sheet_mesclar` | sim | sim |
| `sheet_abrir`, `sheet_salvar` (.fshz, .xlsx, .csv/.tsv) | sim | sim |
| `sheet_selecionar`, `sheet_comando`, `sheet_tela`, `sheet_ver` (foto PNG) | — | sim |

Ao vivo, `sheet_comando` já alcança **qualquer item dos menus** (`menu` pelo rótulo) e **qualquer
atalho** (`tecla`). Isso cobre muita coisa de forma indireta, mas sem parâmetros: abre a janela
ou aplica o padrão, e a IA não consegue dizer "borda grossa vermelha" nem funciona sem o app.
Os itens abaixo são as ferramentas **diretas**, com parâmetros e sem depender da tela.

## Falta — formatação e estrutura (prioridade alta)

O que mais se pede depois de dados e fórmulas.

- [x] **`sheet_formatar`** (14/10, `src/formato.rs`): negrito/itálico/sublinhado/tachado, fonte e
      tamanho, cor do texto e do fundo, alinhamento (horizontal, vertical, quebra, rotação, recuo),
      formato de número (moeda, %, data… e casas decimais), bordas (todas, externas, nenhuma, um
      lado da beira, com cor) e `limpar`. Um Ctrl+Z. **Falta:** formato personalizado (o
      `customFormat` é guardado, mas o motor não formata por ele) e espessura/tracejado de borda
      (o motor só tem lado e cor).
- [x] **`sheet_estrutura`** (14/10): inserir/excluir linhas e colunas (fórmulas ajustadas pelo motor,
      mesclagens acompanham; **sem Ctrl+Z**, como no app), ocultar/reexibir, tamanho em pixels,
      autoajuste (ignora células mescladas, como o Excel). Sem o app, recusa inserir/excluir em
      aba com tabela/AutoFiltro: o reflow das tabelas ainda mora na interface (`tabelas/migracao.rs`).
- [x] **`sheet_mesclar`** (14/10): mesclar, centralizar, através, desmesclar. A regra e o JSON do
      `sheet.mc` saíram da interface para `feathersheet_io::mesclas`; o autoajuste foi para
      `feathersheet_core::operacoes::autoajuste`.
- [ ] **`sheet_ordenar` / `sheet_filtrar`**: ordenar por uma ou mais chaves (com cabeçalho),
      AutoFiltro por critério, remover filtro. Hoje só existe via OpenQuery, que grava em outra aba.
- [ ] **`sheet_mover` / `sheet_copiar`**: mover um bloco (o motor já reescreve as referências em
      `mover_intervalo`) e copiar/colar com as opções do Colar especial (valores, formatos,
      transpor, operação).
- [ ] **`sheet_substituir`**: o `sheet_localizar` não substitui; o motor já tem substituir com desfazer.

## Falta — objetos da planilha (prioridade média)

- [ ] **Tabelas** (Ctrl+T): criar, renomear, estilo, faixas, linha de totais, redimensionar.
      Conferir as ações de `app/tabelas.rs` e mover a regra para o motor onde ainda estiver na interface.
- [ ] **Tabela dinâmica**: criar a partir de um intervalo com campos de linhas, colunas e valores
      (agregação), atualizar e editar. O motor já tem `dinamica`.
- [ ] **Gráficos**: criar (tipo, séries, intervalo), mudar o tipo, título e eixos, apagar; listar os
      gráficos da aba. O motor já tem `grafico`. Ao vivo, o `sheet_ver` confere o desenho.
- [ ] **Formatação condicional**: listar regras, criar (operador e valor, fórmula, escala de cor,
      barra de dados, cores alternadas), remover. A regra mora no motor (`condicional`).
- [ ] **Validação de dados**: lista suspensa, número/data entre limites, caixa de seleção; listar e remover.
- [ ] **Links** em células: inserir, remover, listar.
- [ ] **Imagens e botões de macro**: inserir imagem (arquivo ou `=IMAGEM`), botão ligado a uma macro.
- [ ] **Componentes HTML das macros** (formulários): escrever pelo modelo do designer, ligar a um
      módulo. Hoje o `sheet_macro` só cuida dos módulos JS.

## Falta — dados e análise (prioridade média)

- [ ] **Dividir texto em colunas** e **remover duplicados**, com as mesmas opções das janelas.
- [ ] **Atingir meta**, **tabela de dados** e **cenários** (o motor já tem `atingir_meta`).
- [ ] **OpenQuery com fonte externa**: CSV/XLSX de outro arquivo, Web/API (só por pedido explícito,
      regra 5) e consultas guardadas no arquivo com atualização. Hoje `sheet_consulta` lê só
      intervalos da própria pasta e não guarda a consulta.
- [ ] **Juntar duas fontes** no `sheet_consulta` (`juntar`/`acrescentar` precisam do mapa `fontes`,
      que hoje vai vazio).
- [ ] **Conversor VBA → JS**: ler o `vbaProject.bin` de um `.xlsm` e devolver o JS com a lista
      do que não converteu (motor pronto em `feathersheet-vba`, falta a ferramenta).

## Falta — exibição e arquivo (prioridade baixa)

- [ ] **Congelar painéis**, zoom, linhas de grade, tema: hoje só ao vivo, por `menu`/`tecla`
      (são estado da janela, não do arquivo).
- [ ] **Impressão**: área de impressão, quebras, exportar PDF pelo produtor comum da suíte.
- [ ] **Proteção**: proteger aba/intervalo com senha (o bloqueio ainda não existe no app).
- [ ] **Recentes e modelos**: abrir um recente, criar a partir de um modelo da tela inicial.

## Falta — no próprio MCP

- [ ] **Transporte comum da suíte**: `ao_vivo.rs` é cópia adaptada do `feathercad-mcp`.
      Pela regra 3 da raiz, deve virar um crate próprio usado pelos dois (e pelo Docs e Slide).
- [ ] **`sheet_ver` sem o app**: renderizar um intervalo em PNG fora da janela, para conferir o
      visual também no modo arquivo.
- [ ] **Desfazer entre abas**: uma chamada que escreve em duas abas gera dois Ctrl+Z (o histórico
      é por aba). Agrupar quando o motor tiver histórico da pasta.
- [ ] **`sheet_abrir` ao vivo esperando a carga**: hoje responde "abrindo" na hora; o ideal é
      responder quando o arquivo terminar de abrir.
- [ ] **Confirmação para ações destrutivas ao vivo** (excluir aba, limpar área grande): perguntar
      na tela antes, como o app faz com o usuário.
- [ ] **Guias (`prompts`)** como o `cad_projeto` do FeatherCAD: receitas para relatório,
      controle financeiro, cadastro com formulário e macro, painel com gráfico.
- [ ] **Teste automático do modo ao vivo**: hoje testado à mão; falta um teste que suba o app sem
      janela (ou o `HospedeApp` com socket) e converse pelo binário.
- [ ] **Clique real na faixa**: `sheet_comando` dispara as mesmas mensagens dos botões, mas não
      passa pelo botão. Um botão ligado errado ainda passaria despercebido; só o `sheet_ver` mostra.

## Ordem sugerida

1. ~~`sheet_formatar`, `sheet_estrutura`, `sheet_mesclar`~~ — feitos em 14/10.
2. `sheet_ordenar`/`sheet_filtrar`, `sheet_substituir`, `sheet_mover`/`sheet_copiar`; levar o
   reflow das tabelas para fora da interface (destrava `sheet_estrutura` sem o app em aba com tabela).
3. Gráfico, tabela dinâmica e formatação condicional (reaproveitam o motor que já existe).
4. Transporte comum da suíte (antes de o Docs/Slide ganharem MCP ao vivo).
5. O restante, conforme o uso pedir.
