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
    description: 'A Clean One Page Resume With Contact Details, Experience, And Education.',
    builtin: true,
    blocks: [
      para('Your Full Name', { style: 'Title', align: 'center' }),
      para('City, State  |  email@example.com  |  (555) 555-5555  |  linkedin.com/in/yourname', { align: 'center' }),
      heading('Professional Summary', 2),
      para('Write A Two To Three Sentence Summary That Highlights Your Experience, Key Skills, And Career Goals.'),
      heading('Work Experience', 2),
      para('Job Title | Company Name | City, State', { style: 'Heading3' }),
      para('Month Year - Present'),
      para('Describe Your Role, Key Achievements, And Impact. Use Action Verbs And Quantify Results Where Possible.'),
      para('Job Title | Previous Company | City, State', { style: 'Heading3' }),
      para('Month Year - Month Year'),
      para('Describe Your Responsibilities And Notable Contributions.'),
      heading('Education', 2),
      para('Degree Name | University Name | City, State', { style: 'Heading3' }),
      para('Graduation Year'),
      heading('Skills', 2),
      para('List Relevant Technical, Professional, And Soft Skills Separated By Commas.')
    ]
  },
  {
    id: 'letter',
    name: 'Business Letter',
    description: 'A Formal Letter With Date, Addresses, Salutation, Body, And Signature Line.',
    builtin: true,
    blocks: [
      para('Your Name', { style: 'Heading3' }),
      para('Your Address'),
      para('City, State ZIP Code'),
      para(''),
      para('Month Day, Year', { align: 'right' }),
      para(''),
      para('Recipient Name'),
      para('Company Or Organization'),
      para('Address'),
      para('City, State ZIP Code'),
      para(''),
      para('Dear Recipient Name,'),
      para(''),
      para('Write The Purpose Of Your Letter In The First Paragraph. Keep Your Tone Professional And Courteous.'),
      para(''),
      para('Use Additional Paragraphs To Provide Supporting Details, Next Steps, Or A Call To Action.'),
      para(''),
      para('Sincerely,'),
      para(''),
      para(''),
      para('Your Signature'),
      para('Your Printed Name')
    ]
  },
  {
    id: 'apa-paper',
    name: 'APA Style Paper',
    description: 'An APA Formatted Paper With Title Page Heading, Abstract, Body, And References.',
    builtin: true,
    blocks: [
      para('Your Paper Title', { style: 'Title', align: 'center' }),
      para('Your Name'),
      para('University Or Institution'),
      para('Course Name And Number'),
      para('Instructor Name'),
      para('Due Date'),
      para('Abstract', { style: 'Heading2' }),
      para('Write A 150 To 250 Word Abstract That Summarizes Your Research Question, Methods, Key Findings, And Conclusions.'),
      para('Keywords: keyword one, keyword two, keyword three', { style: 'Heading3' }),
      heading('Introduction', 1),
      para('Introduce Your Topic, Provide Background Context, And State Your Thesis Or Research Question.'),
      heading('Method', 1),
      para('Describe How You Conducted Your Research, Including Participants, Materials, And Procedure.'),
      heading('Results', 1),
      para('Summarize The Main Findings Of Your Study Without Interpretation.'),
      heading('Discussion', 1),
      para('Interpret Your Results, Discuss Their Implications, Acknowledge Limitations, And Suggest Future Research.'),
      heading('References', 1),
      para('List Your Sources In APA Format, One Per Line, Alphabetized By Author Last Name.')
    ]
  },
  {
    id: 'meeting-notes',
    name: 'Meeting Notes',
    description: 'A Structured Template For Agenda, Discussion Points, Decisions, And Action Items.',
    builtin: true,
    blocks: [
      para('Meeting Title', { style: 'Title' }),
      para('Date: Month Day, Year    Time: HH:MM    Location: Room Or Video Call'),
      para('Attendees: Name One, Name Two, Name Three'),
      heading('Agenda', 2),
      para('1. Review Previous Action Items', { list: { ordered: true, level: 0 } }),
      para('2. Topic One Discussion', { list: { ordered: true, level: 0 } }),
      para('3. Topic Two Discussion', { list: { ordered: true, level: 0 } }),
      para('4. Questions And Answers', { list: { ordered: true, level: 0 } }),
      heading('Discussion Notes', 2),
      para('Summarize The Key Points Raised During Each Agenda Item.'),
      heading('Decisions', 2),
      para('List Any Decisions Made During The Meeting.'),
      heading('Action Items', 2),
      para('Task Description | Owner | Due Date', { style: 'Heading3' }),
      para('Describe The Task | Name | Month Day, Year'),
      heading('Next Meeting', 2),
      para('Proposed Date And Time:')
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
