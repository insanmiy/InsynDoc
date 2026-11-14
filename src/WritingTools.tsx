import { useEffect, useState } from 'react';
import { useDocxEditor } from '@docx-editor.dev/react';
import { Modal } from './Modal';
import { linearMathToOmml } from '@docx-editor.dev/core/store';

export type WritingTool = 'date' | 'symbols' | 'table' | 'math';

const symbols = [
  ['©', 'Copyright'], ['®', 'Registered trademark'], ['™', 'Trademark'], ['§', 'Section'], ['¶', 'Paragraph'],
  ['•', 'Bullet'], ['…', 'Ellipsis'], ['–', 'En dash'], ['—', 'Em dash'], ['“', 'Opening double quote'], ['”', 'Closing double quote'],
  ['‘', 'Opening single quote'], ['’', 'Closing single quote'], ['€', 'Euro'], ['£', 'Pound'], ['¥', 'Yen'], ['¢', 'Cent'],
  ['±', 'Plus or minus'], ['×', 'Multiplication'], ['÷', 'Division'], ['≠', 'Not equal'], ['≤', 'Less than or equal'],
  ['≥', 'Greater than or equal'], ['∞', 'Infinity'], ['√', 'Square root'], ['π', 'Pi'], ['°', 'Degree'],
  ['←', 'Left arrow'], ['→', 'Right arrow'], ['↑', 'Up arrow'], ['↓', 'Down arrow'], ['✓', 'Check mark']
] as const;

const titles: Record<WritingTool, string> = { date: 'Insert date and time', symbols: 'Special characters', table: 'Insert custom table', math: 'Insert math equation' };

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
    if (!result.ok) { setError('This content cannot be inserted at the current selection. Choose an editable location and try again.'); return; }
    onClose();
    requestAnimationFrame(() => editor?.focus());
  }

  function insertText(text: string): void {
    if (!editor) return;
    finishInsert(editor.exec({ type: 'insertText', text }));
  }

  return <Modal open={tool !== null} title={tool ? titles[tool] : 'Writing tools'} onClose={onClose} className="writing-tools-modal">
    {tool === 'math' && <form onSubmit={(event) => {
      event.preventDefault();
      const surface = editor?.surface;
      if (!editor || !surface) return;
      const selection = surface.state().selection;
      if (editor.getActiveScope().kind !== 'body' || selection.anchor.paragraphId !== selection.head.paragraphId || selection.anchor.offset !== selection.head.offset) { setError('Place your cursor in the document body without selecting text.'); return; }
      const parsed = linearMathToOmml(equation, () => `math-${crypto.randomUUID()}`);
      if (!parsed.ok) { setError('This equation could not be read. Try a supported example below.'); return; }
      const result = surface.applyAutomationOps(() => [{ op: 'insertEquation', paragraphId: selection.head.paragraphId, offset: selection.head.offset, nodes: [parsed.equation], length: 1 }]);
      if (!result.committed || result.rejected) { setError('The equation cannot be inserted at this location.'); return; }
      onClose();
      requestAnimationFrame(() => editor.focus());
    }}>
      <p className="modal-description">Create an editable Word equation. Click an inserted equation to edit it.</p>
      <label className="page-input-label" htmlFor="math-expression">Math expression</label>
      <input id="math-expression" autoFocus className="page-number-input" value={equation} onChange={(event) => { setEquation(event.target.value); setError(null); }} maxLength={2000}/>
      <p className="modal-description">Examples: x^2, x_1, (a+b)/c, sqrt(x). Use parentheses to group terms.</p>
      <footer className="modal-footer"><button type="submit" className="primary-button">Insert equation</button></footer>
    </form>}
    {tool === 'date' && <>
      <p className="modal-description">Insert a fixed date or time at your cursor.</p>
      <fieldset className="date-formats"><legend>Choose a format</legend>{dates.map((date, index) => <label key={index}><input type="radio" name="date-format" checked={dateFormat === index} onChange={() => setDateFormat(index)}/><span>{date}</span></label>)}</fieldset>
      <footer className="modal-footer"><button className="primary-button" onClick={() => insertText(dates[dateFormat])}>Insert date and time</button></footer>
    </>}
    {tool === 'symbols' && <>
      <label className="page-input-label" htmlFor="symbol-search">Search by name or character</label>
      <input id="symbol-search" autoFocus className="page-number-input" placeholder="Try copyright, euro, or arrow" value={query} onChange={(event) => setQuery(event.target.value)}/>
      <div className="symbol-grid">{symbols.filter(([symbol, name]) => `${symbol} ${name}`.toLowerCase().includes(query.trim().toLowerCase())).map(([symbol, name]) => <button key={name} title={name} aria-label={`Insert ${name.toLowerCase()}`} onClick={() => insertText(symbol)}>{symbol}</button>)}</div>
      {symbols.every(([symbol, name]) => !`${symbol} ${name}`.toLowerCase().includes(query.trim().toLowerCase())) && <p className="modal-description">No matching characters.</p>}
    </>}
    {tool === 'table' && <form noValidate onSubmit={(event) => {
      event.preventDefault();
      const rowCount = Number(rows);
      const columnCount = Number(columns);
      if (![rowCount, columnCount].every((value) => Number.isInteger(value) && value >= 1 && value <= 30)) { setError('Rows and columns must be whole numbers from 1 to 30.'); return; }
      if (editor) finishInsert(editor.exec({ type: 'insertTable', rows: rowCount, cols: columnCount }));
    }}>
      <p className="modal-description">Choose up to 30 rows and 30 columns. You can adjust the table after inserting it.</p>
      <div className="table-dimensions"><label>Rows<input autoFocus className="page-number-input" type="number" min="1" max="30" value={rows} onChange={(event) => { setRows(event.target.value); setError(null); }}/></label><label>Columns<input className="page-number-input" type="number" min="1" max="30" value={columns} onChange={(event) => { setColumns(event.target.value); setError(null); }}/></label></div>
      <footer className="modal-footer"><button type="submit" className="primary-button">Insert table</button></footer>
    </form>}
    {error && <p className="form-error" role="alert">{error}</p>}
  </Modal>;
}
