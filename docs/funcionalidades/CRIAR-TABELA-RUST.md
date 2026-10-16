# Criar Tabela no FeatherSheet Rust/Iced

Entrega de 08/10/2026. **Implementação publicada na `main`; testes Cargo e smoke visual pendentes de execução no ambiente do mantenedor.**

## Como usar

1. Abra FeatherSheet. Preencha, por exemplo, `A1:B3` com o cabeçalho `Produto, Preço` e dados.
2. Selecione uma célula dentro desse bloco e escolha **Inserir → Tabela** ou **Ctrl+T**. O motor Rust `ops::regiao_atual` reconhece o bloco contíguo; se houver seleção retangular, respeita os limites selecionados.
3. Na caixa **Criar tabela**, confira o intervalo, ajuste o nome e a opção **Minha tabela tem cabeçalhos**. A sugestão inicial usa `ops::parece_cabecalho` (cabeçalho textual e números na linha seguinte).
4. Confirme. O Canvas mostra o cabeçalho azul, negrito branco e faixas de linhas alternadas sem converter dados ou modificar estilos explícitos.
5. Salve como `.fshz`, feche e reabra: o intervalo e a aparência voltam porque os metadados vivem em `Sheet.tb`.

**Em uma célula vazia**, o diálogo propõe uma tabela de **6 linhas por 3 colunas**, com `Coluna1`, `Coluna2` e `Coluna3`. Cabeçalhos só são escritos quando a região está realmente vazia, evitando sobrescrever valores. Se existir dado no retângulo sugerido, a interface pede seleção explícita.

## Contrato e arquivos

- `feathersheet-gui/src/app/tabelas.rs`: cria e valida o metadado `TabelaDaAba` (o mesmo contrato JSON de `feathersheet/src/modelo/tabela.ts`), busca nomes com `feathersheet_core::operacoes::tabela::nome_disponivel`, usa `ops::regiao_atual` e `ops::parece_cabecalho`, rejeita nomes repetidos e tabelas cruzadas. Em `Sheet.tb`, cria:
  `{id,nome,area,temCabecalho,faixas,estilo:{tema:'azul',faixasColuna:false,borda:true},filtros:[]}`.
- `feathersheet-gui/src/app.rs`: mensagens de abrir, nome, cabeçalho, criar e cancelar, diálogo em overlay Iced, atalho Ctrl+T e testes de interface/arquivo.
- `feathersheet-gui/src/app/faixa_nativa.rs`: botão **Inserir → Tabela** funcional.
- `feathersheet-gui/src/grade.rs`: interpreta regiões do JSON **uma vez por desenho do Canvas**, colore somente células visíveis, mantém formatação condicional e estilos explícitos com precedência e não percorre 100 milhões de linhas para aplicar cores.
- O motor colunar (`DataTable`, `FormulaTable`) e os codecs FSHZ não foram substituídos.

## Migração posterior das operações TS

Após esta primeira entrega, foram incorporados: renomeação, conversão em
intervalo, seis temas, faixas por linha/coluna, borda, linha e função de totais,
ordenação, AutoFiltro (Ctrl+Shift+L), filtros por regra/lista com pesquisa,
crescimento automático e reajuste após mudanças estruturais. Os controles visuais e propriedades agora são apresentados na **aba
contextual Design da Tabela**, visível somente ao selecionar uma célula de
tabela; a seta do cabeçalho abre um **menu suspenso compacto de filtro**
junto à coluna, sem o painel lateral antigo.

O limite artificial de 128 colunas foi removido; o motor utiliza os limites
reais do FeatherSheet (100 milhões de linhas e 16.384 colunas).

**Os testes Cargo e o aceite visual ainda estão pendentes.** O popover TS
possui opções não reproduzidas integralmente; totais sobre milhões de linhas
ainda precisam de cache por revisão e o desfazer dos metadados requer
uma etapa dedicada.

[Ver matriz de paridade e procedimentos completos](TABELAS-TS-PARA-RUST.md).

## COMO TESTAR NO MEU COMPUTADOR

Na raiz do projeto:

```bash
git pull --ff-only origin main
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml --all-targets -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml tabelas::testes -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml criar_tabela_ -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```

**Resultados esperados:** `cargo check` sem erros, aprovação dos testes
`tabelas::testes` e `criar_tabela_`, botão Inserir → Tabela funcional,
sem alteração dos dados existentes, com persistência de metadados FSHZ.
Status de aceite permanece **pendente** até o mantenedor executar os comandos.
