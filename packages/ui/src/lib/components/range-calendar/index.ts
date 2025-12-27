import Cell from '$lib/components/range-calendar/range-calendar-cell.svelte';
import Day from '$lib/components/range-calendar/range-calendar-day.svelte';
import GridRow from '$lib/components/range-calendar/range-calendar-grid-row.svelte';
import Grid from '$lib/components/range-calendar/range-calendar-grid.svelte';
import HeadCell from '$lib/components/range-calendar/range-calendar-head-cell.svelte';
import Header from '$lib/components/range-calendar/range-calendar-header.svelte';
import Heading from '$lib/components/range-calendar/range-calendar-heading.svelte';
import Months from '$lib/components/range-calendar/range-calendar-months.svelte';
import NextButton from '$lib/components/range-calendar/range-calendar-next-button.svelte';
import PrevButton from '$lib/components/range-calendar/range-calendar-prev-button.svelte';
import Root from '$lib/components/range-calendar/range-calendar.svelte';
import { RangeCalendar as RangeCalendarPrimitive } from 'bits-ui';

const GridHead = RangeCalendarPrimitive.GridHead;
const GridBody = RangeCalendarPrimitive.GridBody;

export {
	Day,
	Cell,
	Grid,
	Header,
	Months,
	GridRow,
	Heading,
	GridBody,
	GridHead,
	HeadCell,
	NextButton,
	PrevButton,
	//
	Root as RangeCalendar,
};
