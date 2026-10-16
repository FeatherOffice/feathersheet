# Trabalho dividido com o Gemini — migração para o Rust

**Direção atual — 07/10/2026: motor e interface 100% Rust, com Iced.**
Prioridade: **FeatherSlide → FeatherDocs → FeatherSheet**; FeatherCAD
depois, se houver tempo. O inventário abaixo registra funcionalidades e
decisões anteriores; a migração da interface deste aplicativo ainda deve
ser conferida no código, sem presumir que os recursos já existem em Iced.

Combinado de 21/09/2026: a divisão é **por arquivo** (regra geral em
`CLAUDE.md` → "Trabalho em paralelo com o Gemini", sempre a versão
atualizada de quem é dono de quê). Este documento lista as tarefas já
combinadas com o Gemini e o estado de cada uma. As instruções completas
("para colar") de tarefas **concluídas** foram resumidas — o texto original,
se precisar, está no histórico do git (`git log --follow -- TAREFAS-GEMINI.md`).

## Regras que valem para toda tarefa aqui

- **Ninguém acrescenta funcionalidade nova em `src/core/`** — vai ser
  reescrito, então o que entra ali nasce para o lixo e ainda é diferença
  nova para o Rust perseguir.
- **Pronto** = `cargo test` e `cargo clippy --all-targets` limpos, dentro do
  crate da tarefa (`sheet-core` ou `sheet-io`).
- **Um arquivo, um dono** — inclusive `lib.rs`, que é sempre do Claude (ele
  liga o `pub mod` antes de entregar a tarefa, para ninguém brigar por essa
  linha).

## Concluído

- **Conflito de arquitetura do armazenamento** (21/09/2026): o Gemini
  adiantou `sheet.rs` com `HashMap<u64, f64>` (só número) enquanto o Claude
  construía `pool`/`column_store`/`data_table` (colunar, com texto e
  booleano) em paralelo, sem um saber do outro. **O usuário decidiu**: o
  armazenamento colunar venceu. `sheet.rs` foi trocado por dentro para usar
  `DataTable` — a API pública não mudou. Detalhe no comentário de topo de
  `crates/sheet-core/src/planilha/sheet.rs`.
- **G7–G11** (triviais, sem depender de `Sheet`): `locale.rs`, `sheet-io::
  recents`, `formula_text.rs`, `style.rs` (formatação de número sem
  `Intl`, à mão), `image.rs` (só o modelo).
- **G14–G21 — o resto das ~161 funções do motor**: todas ligadas na tabela
  `FUNCOES` de `crates/sheet-core/src/funcoes/functions.rs` — condicionais
  (`SUMIFS`/`COUNTIFS`/…), datas (`fn_date.rs`), lógica/informação
  (`fn_info.rs`), o resto de matemática/texto/estatística/procura/matriz
  dinâmica. Catálogo completo, com descrição e apelido: `docs/funcionalidades/FUNCOES.md`.
- **Item 2b — gravar o `.plz` sem célula atravessar a ponte**: é a raiz do
  `crates/sheet-io/src/plz_write.rs` de hoje.
- **O gráfico pisca em branco ao arrastar/redimensionar** (revisado
  25/09/2026, já resolvido): `setupDrag` só mexe em `box.style.left`/`top`
  no `mousemove` e só chama `sheet.updateChart`/`onChanged` no `mouseup`;
  `setupResize` desenha direto no mesmo canvas (`drawChart`), sem
  `replaceChildren` — que só acontece em `render()`, fora dos dois
  caminhos. O commit que resolveu não citou esta tarefa; documento estava
  desatualizado.
- **G13 — arrastar a referência da fórmula** (27/09/2026, feito pelo
  Claude): `crates/sheet-core/src/formula/ref_drag.rs` (`box_at`,
  `move_rect`, `resize_rect`, `boxes_for_sheet`, `drag_rect`; o texto
  reaproveita o `assistente.rs`). Ligado: `formula_arrastar`,
  `formula_caixas_visiveis`.

## Se o Gemini não estiver confortável em Rust

A divisão vira a de sempre: Gemini fica em
`src/ui/` com o que sobrevive à migração (painéis, acabamento do
OpenQuery, impressão, filtros).
