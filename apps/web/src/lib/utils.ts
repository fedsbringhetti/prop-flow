import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

/** Compone classi Tailwind risolvendo i conflitti a favore dell'ultima. */
export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs))
}
