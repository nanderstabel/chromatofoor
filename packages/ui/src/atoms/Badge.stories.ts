import type { Meta, StoryObj } from '@storybook/svelte';
import Badge from './Badge.svelte';

const meta = {
  title: 'Atoms/Badge',
  component: Badge,
  tags: ['autodocs'],
  argTypes: {
    variant: {
      control: { type: 'select' },
      options: ['command', 'query', 'event', 'entity'],
    },
  },
} satisfies Meta<Badge>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Command: Story = {
  args: {
    variant: 'command',
  },
};

export const Query: Story = {
  args: {
    variant: 'query',
  },
};

export const Event: Story = {
  args: {
    variant: 'event',
  },
};

export const Entity: Story = {
  args: {
    variant: 'entity',
  },
};
