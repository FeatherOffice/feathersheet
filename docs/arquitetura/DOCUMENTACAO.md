# OpenSheet — como funciona de ponta a ponta

**Direção atual — 07/10/2026: motor e interface 100% Rust, com Iced.**
Prioridade: **FeatherSlide → FeatherDocs → FeatherSheet**; FeatherCAD
depois, se houver tempo. O inventário abaixo registra funcionalidades e
decisões anteriores; a migração da interface deste aplicativo ainda deve
ser conferida no código, sem presumir que os recursos já existem em Iced.

Documento de contexto, não de contrato — `ARQUITETURA.md` é o contrato
(quem é dono do quê), `ROADMAP.md` é o inventário do que já existe,
`PLANO-BACKEND.md` é o mapa arquivo-por-arquivo da migração. Este aqui
existe para uma pessoa (ou uma sessão nova) entender **como o sistema
funciona de verdade, com código real**, rápido o bastante para propor uma
ideia nova sem ficar um dia lendo fonte. Reescrito em 25/09/2026 — a
versão anterior descrevia a arquitetura de antes da virada para Rust
(21/09/2026) e estava com os papéis invertidos.

## 3. A ponte: um comando, um corpo JSON, bytes voltam — síncrono

No Rust, todo comando passa por um despacho único
(`crates/sheet-ponte/src/lib.rs::Ponte::chamar`), que casa a string do
nome com a função:

```rust
"bloco" => {
    let i = self.indice(&a)?;
    Ok(sheet_core::planilha::bloco::montar(&self.pasta, i,
        a.u32("r0")?, a.u32("c0")?, a.u32("r1")?, a.u32("c1")?, ...))
}
"celula_gravar" => { ... }
"ordenar" => { ... }
```

Cerca de 50 comandos hoje (`celula_gravar`, `aba_inserir`, `ordenar`,
`localizar`, `desfazer`, `refazer`, `estatisticas_coluna`,
`valores_distintos`, `condicional_bloco`, `mover_intervalo`, `bloco`…) —
a lista completa é o próprio `match` em `sheet-ponte/src/lib.rs`, e é o
primeiro lugar a olhar quando a pergunta é "existe um comando pra isso".

## 5. Editar uma célula: comando, não objeto

Digitar numa célula e apertar Enter não monta um objeto `Cell` em
JavaScript. Vira `celula_gravar` (ou `celulas_aplicar`, em lote) pela
ponte, o Rust recalcula o que depende daquilo, e devolve **a versão nova
da aba e os retângulos que sujaram** — não o conteúdo. A grade lê os
retângulos sujos de novo (item 4) na próxima vez que precisar deles; se
não estão na tela, nem isso.

## 6. O motor de fórmula

Tokenizer → parser (precedência de operador, Pratt) → AST → avaliador. O
avaliador **não sabe onde a célula mora** — tudo que precisa da planilha
passa por um `trait`:

```rust
// crates/sheet-core/src/formula/evaluator.rs
pub trait EvalContext {
    fn get_value(&self, row: u32, col: u32, sheet: Option<&str>) -> Value;
    fn for_each_in_range(&self, rect: &Rect, visit: &mut dyn FnMut(&Value, u32, u32));
    fn range_size(&self, rect: &Rect) -> u64;
    fn resolve_name(&self, name: &str) -> Option<Rect>;
}
```

Isso é o que deixa o mesmo avaliador servir um teste (planilha de mentira
que implementa o `trait`) e a planilha de verdade — e é o que faz `SOMA`
de uma coluna inteira (`A:A`) não custar 100 milhões de leituras: quem
implementa `for_each_in_range` (a `Sheet` de verdade) já sabe pular
direto para as células que existem.

Uma função da planilha tem esta forma — `fn_sum` inteira, sem cortar nada:

```rust
// crates/sheet-core/src/funcoes/fn_math.rs
pub fn fn_sum(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    match collect(args, ev) {
        Ok(nums) => Computed::Scalar(Value::Number(nums.iter().sum())),
        Err(e) => Computed::Scalar(Value::Error(e)),
    }
}
```

Os argumentos chegam **como árvore, não avaliados** (por isso `SE` não
calcula o ramo que não usa). ~164 funções assim, cada uma com nome
canônico em inglês e apelidos em português — catálogo completo, gerado do
próprio código-fonte: **`docs/funcionalidades/FUNCOES.md`**.

## 7. As quatro moradas de uma célula

Não existe um único `HashMap<Cell>`. `Sheet::get_value` (o coração do
armazenamento) mostra as três primeiras:

```rust
pub fn get_value_em(&self, row: u32, col: u32, host: Option<&dyn Hospedeiro>) -> Value {
    let k = Self::cell_key(row, col);
    if let Some(cell) = self.cells.get(&k) {
        // fórmula com intervalo, outra aba, matriz, volátil ou com estilo —
        // fica como objeto (árvore + valor) porque não dá pra virar padrão
        match &cell.formula {
            Some(node) => { /* recalcula se sujo, lê do cache */ }
            None => cell.value.clone(),  // erro de compilação, já pronto
        }
    } else if self.formulas.has(row, col) {
        self.recalc_formula_table_if_needed(row, col, host)  // fórmula em padrão
    } else {
        self.data.get_cell_value(row, col, &self.pool)  // literal, colunar
    }
}
```

- **`cells`** (`HashMap`) — só fórmula que não cabe em padrão (intervalo,
  outra aba, matriz, volátil, com estilo direto) e célula vazia-com-só-estilo.
- **`formulas`** (`FormulaTable`) — a maioria das fórmulas: uma coluna
  preenchida para baixo (`=B2+C2`, `=B3+C3`…) é **um padrão**, guardado uma
  vez; por célula ficam só 12 bytes (número do padrão + valor calculado).
  Não guarda texto — reconstrói do padrão quando alguém pede.
- **`data`** (`DataTable`, colunar) — literal (número, texto, booleano) sem
  fórmula. É o que faz uma planilha de milhões de linhas caber na memória.
- **`styles`** — separado, porque célula com conteúdo e célula com estilo
  são independentes (uma coluna toda em negrito sem dado nenhum digitado).

Quem decide onde uma célula cai é a análise de padrão
(`patternCheck`/`PatternCheck`), que roda **uma vez por padrão**, não por
célula. Detalhe e a motivação completa (a planilha de 500 mil linhas que
foi de 1,5 GB para 30 MB): `ROADMAP.md` → "Fórmula por padrão".

## 9. Onde cada funcionalidade grande mora

"Ligada na ponte" é o comando existir e funcionar no Rust; **não** garante
que a interface já o chama — duas linhas abaixo (assistente de fórmula e
OpenQuery) têm comando pronto com nada em `src/ui/`/`src/openquery/`
chamando ainda.

| Funcionalidade | Motor (Rust) | Ligada na ponte? |
| --- | --- | --- |
| Fórmulas, ~164 funções | `funcoes/`, `formula/` | ✅ |
| Ordenar, localizar/substituir, duplicadas, dividir texto | `operacoes/ops.rs` | ✅ |
| Formatação condicional | `operacoes/rules.rs` | ✅ (`condicional_bloco`) |
| Tabela dinâmica | `dinamica/pivot.rs` | ✅ ligada ponta a ponta (`dinamica_aplicar`/`dinamica_campos`/`campo_calculado_previa`) |
| Validação de célula | `operacoes/validation.rs` | 🟡 portado, sem comando ainda |
| Perfil de coluna | `operacoes/perfil.rs` | ✅ (`perfil_coluna`) |
| Mover célula arrastando | `planilha/mover.rs` | ✅ |
| Referência a outro arquivo (`[arquivo]Aba!A1`) | `planilha/vinculo.rs` | ✅ |
| Filtro de tabela | `operacoes/filtro.rs` | ✅ |

Status sempre atualizado, arquivo por arquivo: **`docs/arquitetura/PLANO-BACKEND.md`**.

## 10. Como propor uma ideia nova (guia prático)

- **Algo que lê muita célula por quadro** → não pode ser um comando por
  célula (regra 16 de `CLAUDE.md`, o caso real: totais de tabela pedindo
  31 mil blocos numa tela). Precisa ser **um** comando que devolve o
  agregado pronto (como `estatisticas_coluna`, `valores_distintos`).
- **Conferir o que já existe antes de propor** → `docs/planejamento/ROADMAP.md`
  (inventário) e `docs/funcionalidades/FUNCOES.md` (funções) evitam redescobrir algo que
  já está pronto.
