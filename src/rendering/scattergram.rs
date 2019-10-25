pub struct Scattergram {
    pub parent_id: String,
    pub width: f64,
    pub height: f64,
    pub data_x: Vec<Vec<f64>>,
    pub data_y: Vec<Vec<f64>>,
    pub series_labels: Vec<String>,
    pub x_axis_label: String,
    pub y_axis_label: String,
    pub draw_trendline: bool,
    pub draw_gridlines: bool,
    pub draw_legend: bool,
    pub draw_grey_background: bool,
}

impl Scattergram {
    pub fn get_svg(&self) -> String {
        let data_x2 = format!("{:?}", self.data_x);
        let data_y2 = format!("{:?}", self.data_y);
        let series_labels2 = format!("{:?}", self.series_labels);
        let mut s = String::new();
        s.push_str(&format!(
            "
    <script>
      var plot = {{
        dataX: {},
        dataY: {},
        seriesLabels: {},
        xAxisLabel: \"{}\",
        yAxisLabel: \"{}\",
        width: {},
        height: {},
        drawTrendline: {},
        drawGridlines: {},
        drawLegend: {},
        drawGreyBackground: {},
        parentId: \"{}\"
      }};",
            data_x2,
            data_y2,
            series_labels2,
            self.x_axis_label,
            self.y_axis_label,
            self.width,
            self.height,
            self.draw_trendline,
            self.draw_gridlines,
            self.draw_legend,
            self.draw_grey_background,
            self.parent_id
        ));

        s.push_str(&r#"
      function update(svg) {
        // which of the series labels is longest?
        var maxSeriesLabelLength = 0;
        for (a = 0; a < plot.seriesLabels.length; a++) {
          var sl = plot.seriesLabels[a];
          if (sl.length > maxSeriesLabelLength) { maxSeriesLabelLength = sl.length; }
        }
        var plotLeftMargin = 70.0;
        var plotRightMargin = plot.drawLegend ? 65.0 + maxSeriesLabelLength * 7 : 50.0;
        var plotBottomMargin = 70.0;
        var plotTopMargin = 40.0;
        var plotWidth = plot.width - plotLeftMargin - plotRightMargin;
        var plotHeight = plot.height - plotBottomMargin - plotTopMargin;
        var originX = plotLeftMargin;
        var originY = plotTopMargin + plotHeight;
        var tickLen = 8.0;
        var minorTickLen = tickLen * 0.65;

        // colors
        var lineColor = '#47a3ff'; //'#377eb8'; // '#729ece'; // '#1f77b4'; //'#47a3ff'; //'rgb(2,145,205)';
        var highlightColor = '#ff7f00';
        var btnColor = 'rgb(170,170,170)';
        var btnHoverColor = 'rgb(150,150,150)';
        var plotBackgroundColor = 'rgb(255,255,255)';
        if (plot.drawGreyBackground) {
          plotBackgroundColor = '#DDD';
        }
        var chartBackgroundColor = 'white';
        // var gridLineColor = 'rgb(120,120,120)';
        // if (plot.drawGreyBackground) {
          var gridLineColor = '#EEE';
        // }
        var trendlineColor = 'DimGray';
        if (plot.drawGreyBackground) {
          trendlineColor = 'DimGray';
        }
        var showValueClr = "black";
        // if (plot.drawGreyBackground) {
        //   showValueClr = '#FFF';
        // }

        // Gridlines
        // var gridlineDash = '1, 5';
        // if (plot.drawGreyBackground) {
          var gridlineDash = 'none';
        // }

        var tableau20 = [[31, 119, 180], [255, 127, 14],
             [44, 160, 44], [214, 39, 40],
             [148, 103, 189], [140, 86, 75],
             [227, 119, 194], [127, 127, 127],
             [188, 189, 34], [23, 190, 207]];

        var regularOpacity = 0.75;
        var deselectedOpacity = 0.10;

        // create the svg element
        var svgns = "http://www.w3.org/2000/svg";
        if (svg == null) {
          svg = document.createElementNS(svgns, "svg");
        } else {
          while (svg.lastChild) {
              svg.removeChild(svg.lastChild);
          }
        }
        svg.id = "plotSvg";
        svg.setAttribute('width', `${plot.width}`);
        svg.setAttribute('height', `${plot.height}`);
        var div = document.getElementById(plot.parentId);
        if (div != null) {
          div.appendChild(svg);
        } else {
          // add it to the body of the document
          document.querySelector("body").appendChild(svg);
        }

        // how many series are there?
        var numSeries = plot.dataY.length;

        // style
        var style = document.createElement("style");
        let styleString = `
        text {
          font-family:Sans,Arial;
        }
        .axisLabel {
          font-weight: normal;
        }
        .xTickLabel {
          fill: black;
          font-size: 85%;
          font-weight: lighter;
        }
        .yTickLabel {
          fill: black;
          font-size: 85%;
          font-weight: lighter;
        }
        .gridLine {
          stroke: ${gridLineColor};
          stroke-dasharray: ${gridlineDash};
          stroke-width: 0.80;
        }
        .tick {
          stroke: black;
          stroke-width: 0.5;
        }
        #plotBorder {
          fill: none;
          stroke: black;
          stroke-width: 0.5;
        }
        #showValue {
          font-size: 85%;
          fill: ${showValueClr};
        }
        #context-menu {
          position:absolute;
          display:none;
        }
        #context-menu ul {
          list-style:none;
          margin:0;
          padding:0;
          background: #EFEFEF;
          opacity: 0.90;
        }
        #context-menu {
          border:solid 1px #CCC;
        }
        #context-menu li {
          font-family:Sans,Arial;
          font-size: 75%;
          text-align: left;
          color:#000;
          display:block;
          padding:5px 15px;
          border-bottom:solid 1px #CCC;
        }
        #context-menu li:last-child {
          border:none;
        }
        #context-menu li:hover {
          background:#007AFF;
          color:#FFF;
        }
        `;

        var dataPointHoverWidth = 4.0;
        var s;
        for (s = 0; s < numSeries; s++) {
          var clrNum = s % tableau20.length;
          var red = tableau20[clrNum][0];
          var green = tableau20[clrNum][1];
          var blue = tableau20[clrNum][2];
          let clr = `rgb(${red},${green},${blue})`;
          // var w = 0.85;
          // let tlClr = `rgb(${Math.floor(red * w)},${Math.floor(green * w)},${Math.floor(blue * w)})`;

          styleString += `
          // .seriesLine${s} {
          //   fill: none;
          //   stroke-width:1;
          //   stroke: ${clr};
          //   opacity:1.0;
          // }
          // .seriesLine${s}:hover {
          //   fill: none;
          //   stroke-width:2;
          //   stroke: ${clr};
          //   opacity:1.0;
          // }
          .seriesTrendline${s} {
            fill: none;
            stroke-width:1.5;
            stroke-dasharray: none;
            stroke: ${trendlineColor};
            opacity:1.0;
          }
          .seriesTrendline${s}:hover {
            fill: none;
            stroke-width:2.5;
            stroke-dasharray: none;
            stroke: red;
            opacity:1.0;
          }
          .dataPoint${s} {
            fill: ${clr};
            stroke-width:0;
            stroke: ${clr};
            opacity: ${regularOpacity};
          }
          .selectedDataPoint${s} {
            fill: red;
            stroke-width: ${dataPointHoverWidth};
            stroke: red;
            opacity: 1.0;
          }
          `;
        }
        style.innerHTML = styleString;
        svg.appendChild(style);

        // background
        var background = document.createElementNS(svgns, "rect");
        background.setAttribute('width', plot.width);
        background.setAttribute('height', plot.height);
        background.style.fill = chartBackgroundColor;
        svg.appendChild(background);

        // translate the origin point
        var g = document.createElementNS(svgns, "g");
        g.setAttribute('id', 'transform');
        g.setAttribute('transform', `translate(${originX},${originY})`);
        svg.appendChild(g);

        // white plot background
        var plotBackground = document.createElementNS(svgns, "rect");
        plotBackground.setAttribute('id', 'plotBackground');
        plotBackground.setAttribute('x', 0.0);
        plotBackground.setAttribute('y', -plotHeight);
        plotBackground.setAttribute('width', plotWidth);
        plotBackground.setAttribute('height', plotHeight);
        plotBackground.style.fill = plotBackgroundColor;
        plotBackground.style.stroke = "none";
        g.appendChild(plotBackground);

        // what are the min/max values?
        var xMin = Infinity;
        var xMax = -Infinity;
        var yMin = Infinity;
        var yMax = -Infinity;
        var val = 0;
        var maxNumPoints = 0;
        var totalNumPoints = 0;
        for (s = 0; s < numSeries; s++) {
          var numPoints = Math.min(plot.dataX[s].length, plot.dataY[s].length);
          if (numPoints > maxNumPoints) { maxNumPoints = maxNumPoints; }
          totalNumPoints += numPoints;
          if (numPoints < 2) {
            alert("Too few points for line graph");
            return;
          }
          if (plot.dataX[s].length != plot.dataY[s].length) {
            alert("The x and y data arrays are unequal in length.");
          }
          for (a = 0; a < numPoints; a++) {
              val = plot.dataX[s][a];
              if (val < xMin) { xMin = val; }
              if (val > xMax) { xMax = val; }
              val = plot.dataY[s][a];
              if (val < yMin) { yMin = val; }
              if (val > yMax) { yMax = val; }
          }
        }

        // We don't want a data point to fall on the plot border
        xMin -= 0.0000001;
        yMin -= 0.0000001;
        xMax += 0.0000001;
        yMax += 0.0000001;

        // X axis
        var xAxisTickSpacing = 0.0000001;
        var xAxisNumTicks = 1000;
        var xRange = xMax - xMin;
        a = 0;
        while (xAxisNumTicks > 20) {
          if (a % 2 == 0) {
            xAxisTickSpacing *= 5.0;
          } else {
            xAxisTickSpacing *= 2.0;
          }
          a++;
          xAxisNumTicks = Math.ceil(xRange / xAxisTickSpacing);
        }
        xMin = Math.floor(xMin / xAxisTickSpacing) * xAxisTickSpacing;
        xAxisNumTicks = Math.ceil((xMax - xMin) / xAxisTickSpacing);
        xRange = xAxisTickSpacing * xAxisNumTicks;
        var xSigDigits = Math.min(decimalPlaces(xMin+xAxisTickSpacing), decimalPlaces(xMin+2*xAxisTickSpacing)); //Math.round(0.1 / xAxisTickSpacing);
        if (xSigDigits === 0 && (xMax - xMin) < xAxisNumTicks) { xSigDigits += 1; }
        var dominantTick = 0;
        if (decimalPlaces(xMin) > decimalPlaces(xMin+xAxisTickSpacing)) {
          dominantTick = 1;
        }
        for (a = 0; a <= xAxisNumTicks; a++) {
            // grid line
            if (plot.drawGridlines) {
              if (xAxisNumTicks <= 10 || a % 2 == dominantTick) {
                if (a > 0 && a < xAxisNumTicks) {
                  var line = document.createElementNS(svgns, "line");
                  line.setAttribute('x1', (a * xAxisTickSpacing) / xRange * plotWidth);
                  line.setAttribute('y1', 0);
                  line.setAttribute('x2', (a * xAxisTickSpacing) / xRange * plotWidth);
                  line.setAttribute('y2', -plotHeight);
                  line.setAttribute('class', 'gridLine');
                  g.appendChild(line);
                }
              }
            }
            // tick mark
            var line = document.createElementNS(svgns, "line");
            line.setAttribute('x1', (a * xAxisTickSpacing) / xRange * plotWidth);
            line.setAttribute('y1', 0);
            line.setAttribute('x2', (a * xAxisTickSpacing) / xRange * plotWidth);
            if (xAxisNumTicks <= 10 || a % 2 == dominantTick) {
              line.setAttribute('y2', tickLen);
            } else {
              line.setAttribute('y2', minorTickLen);
            }
            line.setAttribute('class', 'tick');
            g.appendChild(line);

            // labels
            if (xAxisNumTicks <= 10 || a % 2 == dominantTick) {
                // tick label
                var label = document.createElementNS(svgns, "text");
                label.setAttribute('x', (a * xAxisTickSpacing) / xRange * plotWidth);
                label.setAttribute('y', tickLen + 5);
                label.setAttribute('text-anchor', 'middle');
                label.setAttribute('dominant-baseline', 'hanging');
                label.setAttribute('class', 'xTickLabel');
                label.innerHTML = `${(xMin + a * xAxisTickSpacing).toFixed(xSigDigits)}`;
                g.appendChild(label);
            }
        }

        // Y axis
        var yAxisTickSpacing = 0.0000001;
        var yAxisNumTicks = 1000;
        var yRange = yMax - yMin;
        a = 0;
        while (yAxisNumTicks > 20) {
          if (a % 2 == 0) {
            yAxisTickSpacing *= 5.0;
          } else {
            yAxisTickSpacing *= 2.0;
          }
          a++;
          yAxisNumTicks = Math.ceil(yRange / yAxisTickSpacing);
        }
        yMin = Math.floor(yMin / yAxisTickSpacing) * yAxisTickSpacing;
        yAxisNumTicks = Math.ceil((yMax - yMin) / yAxisTickSpacing);
        yRange = yAxisTickSpacing * yAxisNumTicks;
        var ySigDigits = Math.min(decimalPlaces(yMin+yAxisTickSpacing), decimalPlaces(yMin+2*yAxisTickSpacing));
        if (ySigDigits === 0 && (yMax - yMin) < yAxisNumTicks) { ySigDigits += 1; }
        dominantTick = 0;
        if (decimalPlaces(yMin) > decimalPlaces(yMin+yAxisTickSpacing)) {
          dominantTick = 1;
        }
        for (a = 0; a <= yAxisNumTicks; a++) {
          // grid line
          if (plot.drawGridlines) {
            if (yAxisNumTicks <= 10 || a % 2 == dominantTick) {
              if (a > 0 && a < yAxisNumTicks) {
                var line = document.createElementNS(svgns, "line");
                line.setAttribute('x1', 0); //(a * xAxisTickSpacing) / xRange * plotWidth);
                line.setAttribute('y1', -(a * yAxisTickSpacing) / yRange * plotHeight);
                line.setAttribute('x2', plotWidth); //(a * xAxisTickSpacing) / xRange * plotWidth);
                line.setAttribute('y2', -(a * yAxisTickSpacing) / yRange * plotHeight); //-plotHeight);
                line.setAttribute('class', 'gridLine');
                g.appendChild(line);
              }
            }
          }
            // tick mark
            var line = document.createElementNS(svgns, "line");
            line.setAttribute('x1', 0);
            line.setAttribute('y1', -(a * yAxisTickSpacing) / yRange * plotHeight);
            if (yAxisNumTicks <= 6 || a % 2 == dominantTick) {
              line.setAttribute('x2', -tickLen);
            } else {
              line.setAttribute('x2', -minorTickLen);
            }
            line.setAttribute('y2', -(a * yAxisTickSpacing) / yRange * plotHeight);
            line.setAttribute('class', 'tick');
            g.appendChild(line);

            // labels
            if (yAxisNumTicks <= 6 || a % 2 == dominantTick) {
                // s += `<text transform="translate(${-tickLen - 5}, ${-(a * yAxisTickSpacing) / maxVal * plotHeight}), rotate(270)" x="0" y="0" text-anchor="middle" dominant-baseline="no-change" class="yTickLabel">${(a * yAxisTickSpacing).toFixed(ySigDigits)}</text>`;
                var label = document.createElementNS(svgns, "text");
                label.setAttribute('transform', `translate(${-tickLen - 5},${-(a * yAxisTickSpacing) / yRange * plotHeight}), rotate(270)`);
                label.setAttribute('x', 0);
                label.setAttribute('y', 0);
                label.setAttribute('text-anchor', 'middle');
                label.setAttribute('dominant-baseline', 'no-change');
                label.setAttribute('class', 'yTickLabel');
                label.innerHTML = `${(yMin + a * yAxisTickSpacing).toFixed(ySigDigits)}`;
                g.appendChild(label);
            }
        }

        // axis labels
        var xLabel = document.createElementNS(svgns, "text");
        xLabel.setAttribute('x', plotWidth / 2.0);
        xLabel.setAttribute('y', tickLen + 25.0);
        xLabel.setAttribute('text-anchor', 'middle');
        xLabel.setAttribute('dominant-baseline', 'hanging');
        xLabel.setAttribute('class', 'axisLabel');
        xLabel.innerHTML = plot.xAxisLabel;
        g.appendChild(xLabel);

        var yLabel = document.createElementNS(svgns, "text");
        yLabel.setAttribute('transform', `translate(${-tickLen - 28.0},${-plotHeight / 2.0}), rotate(270)`);
        yLabel.setAttribute('x', 0);
        yLabel.setAttribute('y', 0);
        yLabel.setAttribute('text-anchor', 'middle');
        yLabel.setAttribute('class', 'axisLabel');
        yLabel.innerHTML = plot.yAxisLabel;
        g.appendChild(yLabel);

        // text to show values when hover over
        var showValue = document.createElementNS(svgns, "text");
        showValue.setAttribute('x', 10);
        showValue.setAttribute('text-anchor', 'start');

        // draw the data
        var g2 = document.createElementNS(svgns, "g");
        g2.setAttribute('id', 'lines');
        g.appendChild(g2);

        var radius = 3.0;
        if (totalNumPoints > 15) { radius = 2.75; }
        if (totalNumPoints > 100) { radius = 2.50; }
        if (totalNumPoints > 1000) { radius = 2.0; }
        if (totalNumPoints > 10000) { radius = 1.75; }
        for (let s = 0; s < numSeries; s++) {
          var numPoints = Math.min(plot.dataX[s].length, plot.dataY[s].length);
          // draw the data points
          for (let a = 0; a < numPoints; a++) {
            let c = document.createElementNS(svgns, "circle");
            c.setAttribute('cx', (plot.dataX[s][a] - xMin) / xRange * plotWidth);
            c.setAttribute('cy', -(plot.dataY[s][a] - yMin) / yRange * plotHeight);
            c.setAttribute('r', radius);
            c.setAttribute('class', `dataPoint${s}`);
            c.addEventListener('mouseover', function() {
              c.setAttribute('class', `selectedDataPoint${s}`);
              c.style.opacity = 1.0;
              var s2;
              for (s2 = 0; s2 < numSeries; s2++) {
                if (s2 != s) {
                  if (plot.drawTrendline) {
                    document.getElementById(`seriesTrendline${s2}`).style.opacity = deselectedOpacity;
                  }
                  var i;
                  x = document.getElementsByClassName(`dataPoint${s2}`);
                  for (i = 0; i < x.length; i++) {
                      x[i].style.opacity = deselectedOpacity;
                  }
                }
              }
              showValue.innerHTML = `x: ${(plot.dataX[s][a]).toFixed(xSigDigits+2)}, y: ${(plot.dataY[s][a]).toFixed(ySigDigits+2)}`;
            }, false);
            c.addEventListener('mouseout', function() {
              c.setAttribute('class', `dataPoint${s}`);
              c.style.opacity = regularOpacity;
              var s2;
              for (s2 = 0; s2 < numSeries; s2++) {
                if (s2 != s) {
                  if (plot.drawTrendline) {
                    document.getElementById(`seriesTrendline${s2}`).style.opacity = 1.0;
                  }
                  var i;
                  x = document.getElementsByClassName(`dataPoint${s2}`);
                  for (i = 0; i < x.length; i++) {
                      x[i].style.opacity = regularOpacity;
                  }
                }
              }
              showValue.innerHTML = "";
            }, false);
            g2.appendChild(c);
          }
        }

        // Draw the trendlne(s)
        if (plot.drawTrendline) {
          // first calculate the trendlines
          for (let s = 0; s < numSeries; s++) {
            var n = Math.min(plot.dataX[s].length, plot.dataY[s].length);
            var sumX = 0.0;
            var sumY = 0.0;
            var sumXY = 0.0;
            var sumXX = 0.0;
            var sumYY = 0.0;
            var a;
            var x, y;
            var minX = Infinity, maxX = -Infinity;
            for (a = 0; a < numPoints; a++) {
              x = plot.dataX[s][a];
              y = plot.dataY[s][a];
              sumX += x
              sumY += y
              sumXY += x*y;
              sumXX += x*x;
              sumYY += y*y;
              if (x < minX) { minX = x; }
              if (x > maxX) { maxX = x; }
            }

            let slope = (n * sumXY - (sumX * sumY)) / (n * sumXX - (sumX * sumX));
            let intercept = (sumY - slope * sumX) / n;
            let r = (n * sumXY - (sumX * sumY)) / ((Math.sqrt(n * sumXX - (sumX * sumX)) * (Math.sqrt(n * sumYY - (sumY * sumY)))));
            let r_sqr = r * r;
            var yMean = sumY / n;
            var xMean = sumX / n;

            var line = document.createElementNS(svgns, "line");
            line.setAttribute('x1', (minX - xMin) / xRange * plotWidth);
            var yHat = -(((slope * minX + intercept) - yMin) / yRange * plotHeight);
            line.setAttribute('y1', yHat);
            line.setAttribute('x2', (maxX - xMin) / xRange * plotWidth);
            yHat = -(((slope * maxX + intercept) - yMin) / yRange * plotHeight);
            line.setAttribute('y2', yHat);
            line.setAttribute('class', `seriesTrendline${s}`);
            line.setAttribute('id', `seriesTrendline${s}`);
            // line.setAttribute('style', 'stroke:black');

            line.addEventListener('mouseover', function() {
              var s2;
              for (s2 = 0; s2 < numSeries; s2++) {
                if (s2 != s) {
                  document.getElementById(`seriesTrendline${s2}`).style.opacity = deselectedOpacity;
                  var i;
                  x = document.getElementsByClassName(`dataPoint${s2}`);
                  for (i = 0; i < x.length; i++) {
                      x[i].style.opacity = deselectedOpacity;
                  }
                }
              }
            showValue.innerHTML = `y = ${(slope).toFixed(xSigDigits+2)}x &times; ${(intercept).toFixed(xSigDigits+2)} (r-sqr = ${(r_sqr).toFixed(xSigDigits+2)})`;
            }, false);
            line.addEventListener('mouseout', function() {
              var s2;
              for (s2 = 0; s2 < numSeries; s2++) {
                if (s2 != s) {
                  document.getElementById(`seriesTrendline${s2}`).style.opacity = 1.0;
                  var i;
                  x = document.getElementsByClassName(`dataPoint${s2}`);
                  for (i = 0; i < x.length; i++) {
                      x[i].style.opacity = regularOpacity;
                  }
                }
              }
              showValue.innerHTML = "";
            }, false);

            g2.appendChild(line);
          }
        }

        // Show value label
        showValue.setAttribute('id', 'showValue');
        showValue.setAttribute('y', -plotHeight + 20);
        g.appendChild(showValue);

        // plot border
        var plotBorder = document.createElementNS(svgns, "rect");
        plotBorder.setAttribute('x', 0);
        plotBorder.setAttribute('y', -plotHeight);
        plotBorder.setAttribute('width', plotWidth);
        plotBorder.setAttribute('height', plotHeight);
        plotBorder.id = "plotBorder";
        g.appendChild(plotBorder);

        // add a legend
        if (plot.seriesLabels.length > 0 && plot.drawLegend) {
          radius = 3.0;
          if (totalNumPoints > 15) { radius = 2.5; }
          var legend = document.createElementNS(svgns, "g");
          legend.setAttribute('id', 'legend');
          g.appendChild(legend);
          for (let s = 0; s < numSeries; s++) {
            var y = -(plotHeight - 35 - 23 * (s+1));

            var c = document.createElementNS(svgns, "circle");
            c.setAttribute('cx', plotWidth+25);
            c.setAttribute('cy', y);
            c.setAttribute('r', radius);
            c.setAttribute('class', `dataPoint${s}`);
            legend.appendChild(c);

            var legendLabel = document.createElementNS(svgns, "text");
            legendLabel.setAttribute('x', plotWidth + 48);
            legendLabel.setAttribute('y', y);
            legendLabel.setAttribute('text-anchor', 'left');
            // legendLabel.setAttribute('text-anchor', 'middle');
            legendLabel.setAttribute('dominant-baseline', 'middle');
            legendLabel.setAttribute('class', 'xTickLabel');
            legendLabel.innerHTML = plot.seriesLabels[s];
            legend.appendChild(legendLabel);
          }
        }

        // Add an invisible context menu to the parentId.
        var cm = document.createElement('div');
        cm.id = 'context-menu';
        cm.className = 'context-menu';
        var list = document.createElement('ul');

        var copyBtn = document.createElement("li");
        copyBtn.innerHTML = "Copy";
        copyBtn.addEventListener('click', function() {
          var content = `<svg xmlns='${svgns}' width='${plot.width}' height='${plot.height}'>\n${svg.innerHTML}\n</svg>`;
          // Create an auxiliary hidden input
          var aux = document.createElement("input");
          // Get the text from the element passed into the input
          aux.setAttribute("value", content);
          // Append the aux input to the body
          document.body.appendChild(aux);
          // Highlight the content
          aux.select();
          // Execute the copy command
          document.execCommand("copy");
          // Remove the input from the body
          document.body.removeChild(aux);
          // Give a notification
          alert("The plot's SVG content has been copied to the clipboard.");
        }, false);
        list.appendChild(copyBtn);

        var gridlineBtn = document.createElement("li");
        var verb = plot.drawGridlines ? "Hide " : "Show ";
        gridlineBtn.innerHTML = verb + "Gridlines";
        gridlineBtn.addEventListener('click', function() {
          plot.drawGridlines = !plot.drawGridlines;
          // update the context menu label
          var verb = plot.drawGridlines ? "Hide " : "Show ";
          gridlineBtn.innerHTML = verb + "Gridlines";
          update(svg);
        }, false);
        list.appendChild(gridlineBtn);

        var legendBtn = document.createElement("li");
        var verb = plot.drawLegend ? "Hide " : "Show ";
        legendBtn.innerHTML = verb + "Legend";
        legendBtn.addEventListener('click', function() {
          plot.drawLegend = !plot.drawLegend;
          // update the context menu label
          var verb = plot.drawLegend ? "Hide " : "Show ";
          legendBtn.innerHTML = verb + "Legend";
          update(svg);
        }, false);
        list.appendChild(legendBtn);

        var trendlineBtn = document.createElement("li");
        var verb = plot.drawTrendline ? "Hide " : "Show ";
        var suffix = (numSeries > 1) ? "s" : "";
        trendlineBtn.innerHTML = verb + "Trendline" + suffix;
        trendlineBtn.addEventListener('click', function() {
          plot.drawTrendline = !plot.drawTrendline;
          // update the context menu label
          var verb = plot.drawTrendline ? "Hide " : "Show ";
          var suffix = (numSeries > 1) ? "s" : "";
          trendlineBtn.innerHTML = verb + "Trendline" + suffix;
          update(svg);
        }, false);
        list.appendChild(trendlineBtn);

        var backgroundColorBtn = document.createElement("li");
        var verb = plot.drawGreyBackground ? "Light " : "Dark ";
        backgroundColorBtn.innerHTML = verb + "Background";
        backgroundColorBtn.addEventListener('click', function() {
          plot.drawGreyBackground = !plot.drawGreyBackground;
          // update the context menu label
          var verb = plot.drawGreyBackground ? "Light " : "Dark ";
          backgroundColorBtn.innerHTML = verb + "Background";
          update(svg);
        }, false);
        list.appendChild(backgroundColorBtn);

        cm.appendChild(list);
        document.getElementById(plot.parentId).appendChild(cm);

        var menu = document.getElementById('context-menu');
        document.onclick = function () {
            menu.style.display = 'none';
        };

        document.getElementById('plotSvg').oncontextmenu = function (evt) {
            evt = (evt) ? evt : ((event) ? event : null);
            var posnX = (evt.pageX) ? evt.pageX : ((evt.offsetX) ? evt.offsetX + 10 : null);
            var posnY = (evt.pageY) ? evt.pageY : ((evt.offsetY) ? evt.offsetY + 10 : null);
            menu.style.left = posnX + 'px';
            menu.style.top = posnY + 'px';
            menu.style.display = 'block';
            if (typeof evt.preventDefault != "undefined") {
                evt.preventDefault();
            } else {
                evt.returnValue = false;
            }
        };
      }

      function decimalPlaces(num) {
        var match = (''+num).match(/(?:\.(\d+))?(?:[eE]([+-]?\d+))?$/);
        if (!match) { return 0; }
        return Math.max(
             0,
             // Number of digits right of decimal point.
             (match[1] ? match[1].length : 0)
             // Adjust for scientific notation.
             - (match[2] ? +match[2] : 0));
      }

      update(null);
    </script>"#);

        s
    }
}
