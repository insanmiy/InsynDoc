import { useEffect, useState } from 'react';
import { useDocxEditor } from '@docx-editor.dev/react';
import { Modal } from './Modal';
import { linearMathToOmml } from '@docx-editor.dev/core/store';

export type WritingTool = 'date' | 'symbols' | 'table' | 'math';

const symbols = [
  ['©', 'Copyright'], ['®', 'Registered Trademark'], ['™', 'Trademark'], ['§', 'Section'], ['¶', 'Paragraph'],
  ['•', 'Bullet'], ['…', 'Ellipsis'], ['–', 'En Dash'], ['—', 'Em Dash'], ['“', 'Opening Double Quote'], ['”', 'Closing Double Quote'],
  ['‘', 'Opening Single Quote'], ['’', 'Closing Single Quote'], ['€', 'Euro'], ['£', 'Pound'], ['¥', 'Yen'], ['¢', 'Cent'],
  ['±', 'Plus Or Minus'], ['×', 'Multiplication'], ['÷', 'Division'], ['≠', 'Not Equal'], ['≤', 'Less Than Or Equal'],
  ['≥', 'Greater Than Or Equal'], ['∞', 'Infinity'], ['√', 'Square Root'], ['π', 'Pi'], ['°', 'Degree'],
  ['←', 'Left Arrow'], ['→', 'Right Arrow'], ['↑', 'Up Arrow'], ['↓', 'Down Arrow'], ['✓', 'Check Mark']
] as const;

const titles: Record<WritingTool, string> = { date: 'Insert Date And Time', symbols: 'Special Characters', table: 'Insert Custom Table', math: 'Insert Math Equation' };

export function WritingTools({ tool, onClose }: { tool: WritingTool | null; onClose: () => void }) {
  const editor = useDocxEditor();
  const [openedAt, setOpenedAt] = useState(() => new Date());
  const [dateFormat, setDateFormat] = useState(0);
  const [query, setQuery] = useState('');
  const [rows, setRows] = useState('3');
  const [columns, setColumns] = useState('3');
  const [error, setError] = useState<string | null>(null);
  const [equation, setEquation] = useState('x^2 + y^2 = z^2');

  useEffect(() => {
    setError(null);
    setQuery('');
    setOpenedAt(new Date());
  }, [tool]);

  const dates = [
    openedAt.toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' }),
    openedAt.toLocaleDateString(),
    `${openedAt.getFullYear()}-${String(openedAt.getMonth() + 1).padStart(2, '0')}-${String(openedAt.getDate()).padStart(2, '0')}`,
    openedAt.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' }),
    openedAt.toLocaleString(undefined, { dateStyle: 'long', timeStyle: 'short' })
  ];

  function finishInsert(result: ReturnType<NonNullable<typeof editor>['exec']>): void {
    if (!result.ok) { setError('This Content Cannot Be Inserted At The Current Selection. Choose An Editable Location And Try Again.'); return; }
    onClose();
    requestAnimationFrame(() => editor?.focus());
  }

  function insertText(text: string): void {
    if (!editor) return;
    finishInsert(editor.exec({ type: 'insertText', text }));
  }

  return <Modal open={tool !== null} title={tool ? titles[tool] : 'Writing Tools'} onClose={onClose} className="writing-tools-modal">
    {tool === 'math' && <form onSubmit={(event) => {
      event.preventDefault();
      const surface = editor?.surface;
      if (!editor || !surface) return;
      const selection = surface.state().selection;
      if (editor.getActiveScope().kind !== 'body' || selection.anchor.paragraphId !== selection.head.paragraphId || selection.anchor.offset !== selection.head.offset) { setError('Place Your Cursor In The Document Body Without Selecting Text.'); return; }
      const parsed = linearMathToOmml(equation, () => `math-${crypto.randomUUID()}`);
      if (!parsed.ok) { setError('This Equation Could Not Be Read. Try A Supported Example Below.'); return; }
      const result = surface.applyAutomationOps(() => [{ op: 'insertEquation', paragraphId: selection.head.paragraphId, offset: selection.head.offset, nodes: [parsed.equation], length: 1 }]);
      if (!result.committed || result.rejected) { setError('The Equation Cannot Be Inserted At This Location.'); return; }
      onClose();
      requestAnimationFrame(() => editor.focus());
    }}>
      <p className="modal-description">Create An Editable Word Equation. Click An Inserted Equation To Edit It.</p>
      <label className="page-input-label" htmlFor="math-expression">Math Expression</label>
      <input id="math-expression" autoFocus className="page-number-input" value={equation} onChange={(event) => { setEquation(event.target.value); setError(null); }} maxLength={2000}/>
      <p className="modal-description">Examples: x^2, x_1, (a+b)/c, sqrt(x). Use Parentheses To Group Terms.</p>
      <footer className="modal-footer"><button type="submit" className="primary-button">Insert Equation</button></footer>
    </form>}
    {tool === 'date' && <>
      <p className="modal-description">Insert A Fixed Date Or Time At Your Cursor.</p>
      <fieldset className="date-formats"><legend>Choose A Format</legend>{dates.map((date, index) => <label key={index}><input type="radio" name="date-format" checked={dateFormat === index} onChange={() => setDateFormat(index)}/><span>{date}</span></label>)}</fieldset>
      <footer className="modal-footer"><button className="primary-button" onClick={() => insertText(dates[dateFormat])}>Insert Date And Time</button></footer>
    </>}
    {tool === 'symbols' && <>
      <label className="page-input-label" htmlFor="symbol-search">Search By Name Or Character</label>
      <input id="symbol-search" autoFocus className="page-number-input" placeholder="Try Copyright, Euro, Or Arrow" value={query} onChange={(event) => setQuery(event.target.value)}/>
      <div className="symbol-grid">{symbols.filter(([symbol, name]) => `${symbol} ${name}`.toLowerCase().includes(query.trim().toLowerCase())).map(([symbol, name]) => <button key={name} title={name} aria-label={`Insert ${name}`} onClick={() => insertText(symbol)}>{symbol}</button>)}</div>
      {symbols.every(([symbol, name]) => !`${symbol} ${name}`.toLowerCase().includes(query.trim().toLowerCase())) && <p className="modal-description">No Matching Characters.</p>}
    </>}
    {tool === 'table' && <form noValidate onSubmit={(event) => {
      event.preventDefault();
      const rowCount = Number(rows);
      const columnCount = Number(columns);
      if (![rowCount, columnCount].every((value) => Number.isInteger(value) && value >= 1 && value <= 30)) { setError('Rows And Columns Must Be Whole Numbers From 1 To 30.'); return; }
      if (editor) finishInsert(editor.exec({ type: 'insertTable', rows: rowCount, cols: columnCount }));
    }}>
      <p className="modal-description">Choose Up To 30 Rows And 30 Columns. You Can Adjust The Table After Inserting It.</p>
      <div className="table-dimensions"><label>Rows<input autoFocus className="page-number-input" type="number" min="1" max="30" value={rows} onChange={(event) => { setRows(event.target.value); setError(null); }}/></label><label>Columns<input className="page-number-input" type="number" min="1" max="30" value={columns} onChange={(event) => { setColumns(event.target.value); setError(null); }}/></label></div>
      <footer className="modal-footer"><button type="submit" className="primary-button">Insert Table</button></footer>
    </form>}
    {error && <p className="form-error" role="alert">{error}</p>}
  </Modal>;
}
