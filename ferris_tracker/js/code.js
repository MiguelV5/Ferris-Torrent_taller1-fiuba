var xValuesHour = ['2022-01-01 00:00:00', '2022-01-01 00:10:00', '2022-01-01 00:15:00', '2022-01-01 00:20:00',
'2022-01-01 00:30:00', '2022-01-01 00:45:00', '2022-01-01 00:55:00', '2022-01-01 01:00:00'];
var yValuesHour = [0, 1, 3, 5, 10, 11, 12, 12];

var xValuesFiveHours = ['2022-01-01 00:00:00', '2022-01-01 01:00:00', '2022-01-01 01:30:00', '2022-01-01 02:00:00',
'2022-01-01 03:00:00', '2022-01-01 03:30:00', '2022-01-01 04:00:00', '2022-01-01 05:00:00'];
var yValuesFiveHours = [0, 10, 15, 22, 28, 33, 40, 43];

var xValuesDay = ['2022-01-01 00:00:00', '2022-01-01 05:00:00', '2022-01-01 11:30:00', '2022-01-01 12:00:00',
'2022-01-01 17:00:00', '2022-01-01 20:30:00', '2022-01-01 22:00:00', '2022-01-01 23:30:00'];
var yValuesDay = [0, 20, 35, 44, 55, 60, 70, 88];

var xValuesThreeDays = ['2022-01-01 00:00:00', '2022-01-01 15:00:00', '2022-01-01 21:30:00', '2022-01-02 12:00:00',
'2022-01-02 17:00:00', '2022-01-02 20:30:00', '2022-01-03 13:00:00', '2022-01-03 23:30:00'];
var yValuesThreeDays = [0, 25, 39, 43, 55, 74, 88, 100];

const lineChart = new Chart("lineChart", {
  type: "line",
  data: {
    labels: xValuesHour,
    datasets: [{
      label: 'Conexiones',
      fill: false,
      backgroundColor: "rgba(0,0,255,1.0)",
      borderColor: "rgba(0,0,255,0.1)",
      data: yValuesHour
    }]
  },
  options: {
    responsive: true,
    legend: {display: true},
    scales: {
      xAxes: [{
        type: 'time',
        distribution: 'linear',
        time: {
          displayFormats: {
            'hour': 'ddd HH:mm a',
            'day': 'MMM DD',
            'week': 'MMM DD',
            'month': 'DD MMM YYYY',
            'quarter': 'MMM YYYY',
            'year': 'MMM YYYY',
         }
        }
      }]
    }
  }
});

const barChart = new Chart("barChart", {
  type: "bar",
  data: {
    labels: xValuesHour,
    datasets: [{
      label: 'Conexiones',
      fill: false,
      backgroundColor: "rgba(0,0,255,1.0)",
      borderColor: "rgba(0,0,255,0.1)",
      data: yValuesHour
    }]
  },
  options: {
    responsive: true,
    legend: {display: true},
    scales: {
      xAxes: [{
        type: 'time',
        distribution: 'linear',
        time: {
          displayFormats: {
            'hour': 'ddd HH:mm a',
            'day': 'HH:mm',
            'week': 'MMM DD',
            'month': 'MMM YYYY',
            'quarter': 'MMM YYYY',
            'year': 'MMM YYYY',
         }
        }
      }]
    }
  }
})

function alertSelectedValueLine() {
  var select = document.getElementById('longTimeLine');
  var text = select.options[select.selectedIndex].text;
  if (text == 'Last hour') {
    lineChart.data.labels = xValuesHour
    lineChart.data.datasets[0].data = yValuesHour
    lineChart.update();
  } else if (text == 'Last five hours') {
    lineChart.data.labels = xValuesFiveHours
    lineChart.data.datasets[0].data = yValuesFiveHours
    lineChart.update();
  } else if (text == 'Last day') {
    lineChart.data.labels = xValuesDay
    lineChart.data.datasets[0].data = yValuesDay
    lineChart.update();
  } else {
    lineChart.data.labels = xValuesThreeDays
    lineChart.data.datasets[0].data = yValuesThreeDays
    lineChart.update();
  }
}

function alertSelectedValueBar() {
  var select = document.getElementById('longTimeBar');
  var text = select.options[select.selectedIndex].text;
  if (text == 'Last hour') {
    barChart.data.labels = xValuesHour
    barChart.data.datasets[0].data = yValuesHour
    barChart.update();
  } else if (text == 'Last five hours') {
    barChart.data.labels = xValuesFiveHours
    barChart.data.datasets[0].data = yValuesFiveHours
    barChart.update();
  } else if (text == 'Last day') {
    barChart.data.labels = xValuesDay
    barChart.data.datasets[0].data = yValuesDay
    barChart.update();
  } else {
    barChart.data.labels = xValuesThreeDays
    barChart.data.datasets[0].data = yValuesThreeDays
    barChart.update();
  }
}