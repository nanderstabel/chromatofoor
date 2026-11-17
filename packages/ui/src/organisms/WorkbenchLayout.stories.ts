import type { Meta, StoryObj } from '@storybook/svelte';
import WorkbenchLayout from './WorkbenchLayout.svelte';

const meta = {
  title: 'Organisms/WorkbenchLayout',
  component: WorkbenchLayout,
  tags: ['autodocs'],
  argTypes: {
    brandName: { control: 'text' },
  },
} satisfies Meta<WorkbenchLayout>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    brandName: 'Chromatofoor',
  },
};
