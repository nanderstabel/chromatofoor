import type { Meta, StoryObj } from '@storybook/svelte';
import FormField from './FormField.svelte';

const meta = {
  title: 'Molecules/FormField',
  component: FormField,
  tags: ['autodocs'],
  argTypes: {
    label: { control: 'text' },
    placeholder: { control: 'text' },
    error: { control: 'text' },
    required: { control: 'boolean' },
  },
} satisfies Meta<FormField>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    label: 'Workspace Name',
    placeholder: 'e.g. My Next Venture',
    required: true,
  },
};

export const WithError: Story = {
  args: {
    label: 'Workspace Slug',
    placeholder: 'my-workspace',
    value: 'invalid slug!',
    error: 'Slug must contain only lowercase alphanumeric characters and hyphens.',
    required: true,
  },
};
