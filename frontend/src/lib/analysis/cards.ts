/// The frame around a section of the analysis pages.
///
/// A phone does not get one. The tab panel already draws a box and pads it, so a card inside
/// that is a second border and a second gutter around content that has no neighbour to be told
/// apart from -- perhaps forty pixels of width spent on saying "this is a thing". From `sm` up
/// there is room for the frame and the page reads better with it.
export const ANALYSIS_CARD =
	'max-w-none border-0 bg-transparent p-0 dark:bg-transparent ' +
	'sm:rounded-lg sm:border sm:bg-white sm:p-4 dark:sm:bg-gray-800';
