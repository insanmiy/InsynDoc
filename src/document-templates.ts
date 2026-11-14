import { buildDocx, type DocxBlock, type DocxParagraph, type DocxRun } from './document-builder';

export interface DocumentTemplate {
  id: string;
  name: string;
  description: string;
  builtin: boolean;
  blocks: DocxBlock[];
}

const run = (text: string): DocxRun[] => text ? [{ text }] : [];
const para = (text: string, extra?: Partial<DocxParagraph>): DocxParagraph => ({ inlines: run(text), ...extra });
const heading = (text: string, level: 1 | 2 | 3 = 1): DocxParagraph => para(text, { style: `Heading${level}` });

export const BUILTIN_TEMPLATES: readonly DocumentTemplate[] = [
  {
    id: 'resume',
    name: 'Resume',
    description: 'A clean one page resume with contact details, experience, and education.',
    builtin: true,
    blocks: [
      para('Your full name', { style: 'Title', align: 'center' }),
      para('City, state  |  email@example.com  |  (555) 555-5555  |  linkedin.com/in/yourname', { align: 'center' }),
      heading('Professional summary', 2),
      para('Write a two to three sentence summary that highlights your experience, key skills, and career goals.'),
      heading('Work experience', 2),
      para('Job title | company name | city, state', { style: 'Heading3' }),
      para('Month year - present'),
      para('Describe your role, key achievements, and impact. Use action verbs and quantify results where possible.'),
      para('Job title | previous company | city, state', { style: 'Heading3' }),
      para('Month year - month year'),
      para('Describe your responsibilities and notable contributions.'),
      heading('Education', 2),
      para('Degree name | university name | city, state', { style: 'Heading3' }),
      para('Graduation year'),
      heading('Skills', 2),
      para('List relevant technical, professional, and soft skills separated by commas.')
    ]
  },
  {
    id: 'letter',
    name: 'Business letter',
    description: 'A formal letter with date, addresses, salutation, body, and signature line.',
    builtin: true,
    blocks: [
      para('Your name', { style: 'Heading3' }),
      para('Your address'),
      para('City, state ZIP code'),
      para(''),
      para('Month day, year', { align: 'right' }),
      para(''),
      para('Recipient name'),
      para('Company or organization'),
      para('Address'),
      para('City, state ZIP code'),
      para(''),
      para('Dear recipient name,'),
      para(''),
      para('Write the purpose of your letter in the first paragraph. Keep your tone professional and courteous.'),
      para(''),
      para('Use additional paragraphs to provide supporting details, next steps, or a call to action.'),
      para(''),
      para('Sincerely,'),
      para(''),
      para(''),
      para('Your signature'),
      para('Your printed name')
    ]
  },
  {
    id: 'apa-paper',
    name: 'APA style paper',
    description: 'An APA formatted paper with title page heading, abstract, body, and references.',
    builtin: true,
    blocks: [
      para('Your paper title', { style: 'Title', align: 'center' }),
      para('Your name'),
      para('University or institution'),
      para('Course name and number'),
      para('Instructor name'),
      para('Due date'),
      para('Abstract', { style: 'Heading2' }),
      para('Write a 150 to 250 word abstract that summarizes your research question, methods, key findings, and conclusions.'),
      para('Keywords: keyword one, keyword two, keyword three', { style: 'Heading3' }),
      heading('Introduction', 1),
      para('Introduce your topic, provide background context, and state your thesis or research question.'),
      heading('Method', 1),
      para('Describe how you conducted your research, including participants, materials, and procedure.'),
      heading('Results', 1),
      para('Summarize the main findings of your study without interpretation.'),
      heading('Discussion', 1),
      para('Interpret your results, discuss their implications, acknowledge limitations, and suggest future research.'),
      heading('References', 1),
      para('List your sources in APA format, one per line, alphabetized by author last name.')
    ]
  },
  {
    id: 'meeting-notes',
    name: 'Meeting notes',
    description: 'A structured template for agenda, discussion points, decisions, and action items.',
    builtin: true,
    blocks: [
      para('Meeting title', { style: 'Title' }),
      para('Date: month day, year    time: HH:MM    location: room or video call'),
      para('Attendees: name one, name two, name three'),
      heading('Agenda', 2),
      para('1. Review previous action items', { list: { ordered: true, level: 0 } }),
      para('2. Topic one discussion', { list: { ordered: true, level: 0 } }),
      para('3. Topic two discussion', { list: { ordered: true, level: 0 } }),
      para('4. Questions and answers', { list: { ordered: true, level: 0 } }),
      heading('Discussion notes', 2),
      para('Summarize the key points raised during each agenda item.'),
      heading('Decisions', 2),
      para('List any decisions made during the meeting.'),
      heading('Action items', 2),
      para('Task description | owner | due date', { style: 'Heading3' }),
      para('Describe the task | name | month day, year'),
      heading('Next meeting', 2),
      para('Proposed date and time:')
    ]
  }
];

const templateBytes = new Map<string, Uint8Array>();

export function builtinTemplateBytes(template: DocumentTemplate): Uint8Array {
  let bytes = templateBytes.get(template.id);
  if (!bytes) {
    bytes = buildDocx(template.blocks);
    templateBytes.set(template.id, bytes);
  }
  return bytes;
}

export function templateDisplayName(name: string): string {
  const trimmed = name.trim();
  return /\.docx$/i.test(trimmed) ? trimmed : `${trimmed || 'Untitled'}.docx`;
}
