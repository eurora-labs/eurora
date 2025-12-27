import Button from '$lib/components/form/form-button.svelte';
import Description from '$lib/components/form/form-description.svelte';
import ElementField from '$lib/components/form/form-element-field.svelte';
import FieldErrors from '$lib/components/form/form-field-errors.svelte';
import Field from '$lib/components/form/form-field.svelte';
import Fieldset from '$lib/components/form/form-fieldset.svelte';
import Label from '$lib/components/form/form-label.svelte';
import Legend from '$lib/components/form/form-legend.svelte';
import * as FormPrimitive from 'formsnap';

const Control = FormPrimitive.Control;

export {
	Field,
	Control,
	Label,
	Button,
	FieldErrors,
	Description,
	Fieldset,
	Legend,
	ElementField,
	//
	Field as FormField,
	Control as FormControl,
	Description as FormDescription,
	Label as FormLabel,
	FieldErrors as FormFieldErrors,
	Fieldset as FormFieldset,
	Legend as FormLegend,
	ElementField as FormElementField,
	Button as FormButton,
};
